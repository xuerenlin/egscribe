use crate::plugin::protocol::{PluginMessage, PluginRequest, PluginResponse};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// 从 `BufRead` 读取一行（以 `\n` 为界），允许非 UTF-8 字节（按 lossy 规则替换），
/// 避免插件 stdout 混入系统编码或二进制片段时整条读取线程退出。
fn read_line_utf8_lossy<R: BufRead>(reader: &mut R) -> std::io::Result<Option<String>> {
    let mut buf = Vec::new();
    let n = reader.read_until(b'\n', &mut buf)?;
    if n == 0 {
        return Ok(None);
    }
    if buf.ends_with(b"\n") {
        buf.pop();
        if buf.ends_with(b"\r") {
            buf.pop();
        }
    }
    Ok(Some(String::from_utf8_lossy(&buf).into_owned()))
}

/// 对端已关闭管道（插件退出、崩溃或主动关掉 stdin）时的常见 I/O 错误。
fn is_stdin_pipe_closed(err: &std::io::Error) -> bool {
    match err.kind() {
        std::io::ErrorKind::BrokenPipe
        | std::io::ErrorKind::ConnectionReset
        | std::io::ErrorKind::ConnectionAborted => true,
        _ => {
            #[cfg(windows)]
            {
                // ERROR_NO_DATA (232)：管道正在被关闭。
                if err.raw_os_error() == Some(232) {
                    return true;
                }
            }
            false
        }
    }
}

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// 插件进程管理器
pub struct PluginProcess {
    /// 进程句柄
    child: Option<Child>,
    /// 消息接收通道
    rx: mpsc::Receiver<PluginMessage>,
    /// 消息发送通道
    tx: mpsc::Sender<PluginMessage>,
    /// 同步 `request()` 期间先入队的消息（例如插件在返回 response 前发送的 command 事件）
    pending: VecDeque<PluginMessage>,
    /// 插件ID
    plugin_id: String,
    /// 是否正在运行
    is_running: bool,
}

impl PluginProcess {
    /// 启动插件进程
    pub fn start(plugin_id: String, exe_path: &str, args: Vec<String>, current_dir: Option<std::path::PathBuf>) -> Result<Self, String> {
        let mut cmd = Command::new(exe_path);
        cmd.args(args);
        if let Some(dir) = current_dir {
            cmd.current_dir(dir);
        }
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        #[cfg(windows)]
        cmd.creation_flags(crate::util::win_exec::CREATE_NO_WINDOW);

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to start plugin process: {}", e))?;
        
        let stdin = child.stdin.take()
            .ok_or_else(|| "Failed to get stdin".to_string())?;
        let stdout = child.stdout.take()
            .ok_or_else(|| "Failed to get stdout".to_string())?;
        
        let (tx, rx) = mpsc::channel::<PluginMessage>();
        let (tx_internal, rx_internal) = mpsc::channel::<PluginMessage>();
        
        // 启动读取线程
        let plugin_id_clone = plugin_id.clone();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let line = match read_line_utf8_lossy(&mut reader) {
                    Ok(Some(s)) => s,
                    Ok(None) => break,
                    Err(e) => {
                        log::error!("Plugin {}: Failed to read stdout: {}", plugin_id_clone, e);
                        break;
                    }
                };
                if line.trim().is_empty() {
                    continue;
                }
                if line.contains('\u{fffd}') {
                    log::debug!(
                        "Plugin {}: stdout line contained non-UTF-8 bytes (decoded with lossy replacement)",
                        plugin_id_clone
                    );
                }
                match PluginMessage::from_json_line(&line) {
                    Ok(msg) => {
                        if let Err(e) = tx.send(msg) {
                            log::error!("Plugin {}: Failed to send message: {}", plugin_id_clone, e);
                            break;
                        }
                    }
                    Err(e) => {
                        log::warn!("Plugin {}: Failed to parse message: {} - {}", plugin_id_clone, line, e);
                    }
                }
            }
            log::info!("Plugin {}: Read thread exited", plugin_id_clone);
        });
        
        // 启动写入线程
        let plugin_id_clone2 = plugin_id.clone();
        let mut writer = BufWriter::new(stdin);
        thread::spawn(move || {
            while let Ok(msg) = rx_internal.recv() {
                match msg.to_json_line() {
                    Ok(json) => {
                        if let Err(e) = writeln!(writer, "{}", json) {
                            if is_stdin_pipe_closed(&e) {
                                log::debug!(
                                    "Plugin {}: stdin closed while writing (plugin ended): {}",
                                    plugin_id_clone2,
                                    e
                                );
                            } else {
                                log::error!("Plugin {}: Failed to write message: {}", plugin_id_clone2, e);
                            }
                            break;
                        }
                        if let Err(e) = writer.flush() {
                            if is_stdin_pipe_closed(&e) {
                                log::debug!(
                                    "Plugin {}: stdin closed while flushing (plugin ended): {}",
                                    plugin_id_clone2,
                                    e
                                );
                            } else {
                                log::error!("Plugin {}: Failed to flush: {}", plugin_id_clone2, e);
                            }
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("Plugin {}: Failed to serialize message: {}", plugin_id_clone2, e);
                    }
                }
            }
            log::info!("Plugin {}: Write thread exited", plugin_id_clone2);
        });
        
        Ok(Self {
            child: Some(child),
            rx,
            tx: tx_internal,
            pending: VecDeque::new(),
            plugin_id,
            is_running: true,
        })
    }
    
    /// 发送消息到插件
    pub fn send(&mut self, msg: PluginMessage) -> Result<(), String> {
        if !self.is_running {
            return Err("Plugin process is not running".to_string());
        }
        self.tx.send(msg)
            .map_err(|e| format!("Failed to send message: {}", e))
    }
    
    /// 接收来自插件的消息
    #[allow(dead_code)]
    pub fn recv(&self) -> Result<PluginMessage, mpsc::RecvError> {
        self.rx.recv()
    }
    
    /// 尝试接收消息（非阻塞）：先出队 `request` 期间暂存的消息，再从通道读取
    pub fn try_recv(&mut self) -> Result<PluginMessage, mpsc::TryRecvError> {
        if let Some(msg) = self.pending.pop_front() {
            return Ok(msg);
        }
        self.rx.try_recv()
    }
    
    /// 发送请求并等待响应
    pub fn request(&mut self, msg: PluginMessage, timeout: Duration) -> Result<PluginResponse, String> {
        let request_id = match &msg {
            PluginMessage::Request(PluginRequest::Init { id, .. }) => id.clone(),
            PluginMessage::Request(PluginRequest::Execute { id, .. }) => id.clone(),
            PluginMessage::Request(PluginRequest::Info { id }) => id.clone(),
            PluginMessage::Request(PluginRequest::Shutdown { id }) => id.clone(),
            PluginMessage::Request(PluginRequest::Notify { id, .. }) => id.clone(),
            _ => return Err("Not a request message".to_string()),
        };
        
        // 发送请求
        self.send(msg)?;
        
        // 等待响应（必须只从 `rx` 读，不能用 `try_recv()`：`try_recv` 会优先消费
        // `pending`，会导致先把事件放进 pending 后又立刻反复弹出同一事件，永远读不到后续 Response）
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            match self.rx.try_recv() {
                Ok(PluginMessage::Response(resp)) => {
                    if resp.id == request_id {
                        return Ok(resp);
                    }
                    self.pending.push_back(PluginMessage::Response(resp));
                }
                Ok(other) => {
                    // 插件可能在返回 response 之前先发 ready/command/notify 等，不能丢弃
                    self.pending.push_back(other);
                }
                Err(mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => {
                    return Err(format!("Failed to receive response: {:?}", e));
                }
            }
        }
        
        Err(format!("Request timeout: {}", request_id))
    }
    
    /// 检查进程是否还在运行
    pub fn is_running(&mut self) -> bool {
        if !self.is_running {
            return false;
        }
        
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(Some(status)) => {
                    log::info!("Plugin {} exited with status: {:?}", self.plugin_id, status);
                    self.is_running = false;
                    false
                }
                Ok(None) => true,
                Err(e) => {
                    log::error!("Plugin {}: Failed to check status: {}", self.plugin_id, e);
                    self.is_running = false;
                    false
                }
            }
        } else {
            false
        }
    }
    
    /// 停止插件进程（非阻塞，在后台线程中等待进程退出）
    pub fn stop(&mut self) -> Result<(), String> {
        if !self.is_running {
            return Ok(());
        }
        
        self.is_running = false;
        
        // 尝试优雅关闭
        // 发送shutdown请求
        let shutdown_id = format!("shutdown_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos());
        let _ = self.send(PluginMessage::Request(PluginRequest::Shutdown {
            id: shutdown_id.clone(),
        }));
        
        // 快速检查进程是否已经退出（非阻塞）
        let process_exited = if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(Some(status)) => {
                    log::info!("Plugin {} exited gracefully with status: {:?}", self.plugin_id, status);
                    true
                }
                Ok(None) => false, // 进程还在运行
                Err(e) => {
                    log::warn!("Plugin {}: Error checking status: {}", self.plugin_id, e);
                    false
                }
            }
        } else {
            true // 没有进程句柄，认为已退出
        };
        
        // 将进程句柄移到后台线程进行清理
        if let Some(mut child) = self.child.take() {
            let plugin_id = self.plugin_id.clone();
            
            // 在后台线程中处理进程终止和回收
            thread::spawn(move || {
                // 如果进程还没退出，先尝试优雅等待（最多1秒）
                if !process_exited {
                    let mut exited = false;
                    for _ in 0..10 {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                log::info!("Plugin {} exited gracefully with status: {:?}", plugin_id, status);
                                exited = true;
                                break;
                            }
                            Ok(None) => {
                                // 进程还在运行，继续等待
                                thread::sleep(Duration::from_millis(100));
                            }
                            Err(e) => {
                                log::warn!("Plugin {}: Error checking status: {}", plugin_id, e);
                                break;
                            }
                        }
                    }
                    
                    // 如果还没退出，强制终止
                    if !exited {
                        log::info!("Plugin {}: Force killing process", plugin_id);
                        let _ = child.kill();
                    }
                }
                
                // 关键：等待进程退出并回收，避免僵尸进程
                // 在后台线程中执行，不会阻塞UI
                match child.wait() {
                    Ok(status) => {
                        log::info!("Plugin {} exited with status: {:?}", plugin_id, status);
                    }
                    Err(e) => {
                        log::warn!("Plugin {}: Error waiting for process: {}", plugin_id, e);
                    }
                }
            });
        }
        
        Ok(())
    }
    
    /// 获取插件ID
    #[allow(dead_code)]
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
}

impl Drop for PluginProcess {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

