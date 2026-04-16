#!/usr/bin/env python3
"""
Simple Plugin - Python 版本
这是一个使用 Python 实现的 egscribe 插件示例，展示了如何创建一个基本的插件。
"""

import json
import sys
from typing import Dict, Any, Optional


def send_message(msg: Dict[str, Any]) -> None:
    """发送消息到 stdout"""
    json_str = json.dumps(msg, ensure_ascii=False)
    print(json_str, flush=True)


def send_response(request_id: str, success: bool, data: Optional[Dict[str, Any]] = None, error: Optional[str] = None) -> None:
    """发送响应消息"""
    response = {
        "type": "response",
        "id": request_id,
        "success": success,
        "data": data,
        "error": error
    }
    send_message(response)


def send_event(event_type: str, **kwargs) -> None:
    """发送事件消息"""
    event = {
        "type": "event",
        "event": event_type,
        **kwargs
    }
    send_message(event)


def handle_init(request_id: str, config: Dict[str, Any]) -> None:
    """处理初始化请求"""
    # 发送就绪事件
    send_event("ready", 
               name="Simple Plugin (Python)",
               version="0.1.0",
               capabilities=["echo", "greet", "get_info"])
    
    # 发送初始化响应
    response_data = {
        "message": "Plugin initialized successfully",
        "config": config
    }
    send_response(request_id, True, response_data)


def handle_execute(request_id: str, command: str, params: Dict[str, Any]) -> None:
    """处理执行命令请求"""
    if command == "echo":
        # echo命令：回显参数
        text = params.get("text", "Hello from plugin!")
        
        response_data = {
            "echoed": text
        }
        send_response(request_id, True, response_data)
        
    elif command == "greet":
        # greet命令：问候
        name = params.get("name", "World")
        greeting = f"Hello, {name}! This is a simple Python plugin demo."
        
        # 发送通知事件
        send_event("notify", level="info", message=greeting)
        
        response_data = {
            "greeting": greeting
        }
        send_response(request_id, True, response_data)
        
    elif command == "get_info":
        # get_info命令：获取插件信息
        response_data = {
            "name": "Simple Plugin (Python)",
            "version": "0.1.0",
            "description": "A simple Python plugin demo for egscribe",
            "author": "egscribe",
            "capabilities": ["echo", "greet", "get_info"],
            "language": "Python"
        }
        send_response(request_id, True, response_data)
        
    else:
        send_response(request_id, False, None, f"Unknown command: {command}")


def handle_info(request_id: str) -> None:
    """处理信息请求"""
    response_data = {
        "name": "Simple Plugin (Python)",
        "version": "0.1.0",
        "description": "A simple Python plugin demo for egscribe",
        "capabilities": ["echo", "greet", "get_info"],
        "language": "Python"
    }
    send_response(request_id, True, response_data)


def handle_shutdown(request_id: str) -> None:
    """处理关闭请求"""
    # 发送关闭通知
    send_event("notify", level="info", message="Plugin is shutting down...")
    
    send_response(request_id, True, None)


def handle_notify(request_id: str, event_type: str, data: Dict[str, Any]) -> None:
    """处理通知事件（来自主程序的事件通知）"""
    if event_type == "line_changed":
        # 处理行内容变化事件
        line_no = data.get("line_no", 0)
        line_text = data.get("line_text", "")
        
        # 截断过长的文本
        preview = line_text[:50] + "..." if len(line_text) > 50 else line_text
        
        send_event("notify", 
                  level="info", 
                  message=f"Received line_changed event: line {line_no} = '{preview}'")
        
    elif event_type == "file_opened":
        # 处理文件打开事件
        file_path = data.get("file_path", "unknown")
        send_event("notify", level="info", message=f"File opened: {file_path}")
        
    elif event_type == "file_saved":
        # 处理文件保存事件
        file_path = data.get("file_path", "unknown")
        send_event("notify", level="info", message=f"File saved: {file_path}")
        
    elif event_type == "file_closed":
        # 处理文件关闭事件
        file_path = data.get("file_path", "unknown")
        send_event("notify", level="info", message=f"File closed: {file_path}")
        
    elif event_type == "cursor_changed":
        # 处理光标位置变化事件
        line_no = data.get("line_no", 0)
        column = data.get("column", 0)
        # 可以记录光标位置，但不发送通知（避免过于频繁）
        # 只在调试模式下记录
        if __debug__:
            send_event("notify", 
                      level="debug", 
                      message=f"Cursor moved to line {line_no}, column {column}")
        
    elif event_type == "selection_changed":
        # 处理选择文本变化事件
        selected_text = data.get("selected_text", "")
        
        if selected_text:
            preview = selected_text[:30] + "..." if len(selected_text) > 30 else selected_text
            send_event("notify", level="info", message=f"Selection changed: '{preview}'")
        
    else:
        # 未知的事件类型
        send_event("notify", level="warning", message=f"Received unknown event type: {event_type}")
    
    # 发送响应（通知不需要响应，但为了协议一致性，发送成功响应）
    send_response(request_id, True, {"event_type": event_type, "handled": True})


def handle_message(msg: Dict[str, Any]) -> bool:
    """处理消息，返回是否应该退出"""
    msg_type = msg.get("type")
    
    if msg_type == "request":
        action = msg.get("action")
        request_id = msg.get("id", "")
        
        if action == "init":
            config = msg.get("config", {})
            handle_init(request_id, config)
            return False  # 继续运行
            
        elif action == "execute":
            command = msg.get("command", "")
            params = msg.get("params", {})
            handle_execute(request_id, command, params)
            return False  # 继续运行
            
        elif action == "info":
            handle_info(request_id)
            return False  # 继续运行
            
        elif action == "shutdown":
            handle_shutdown(request_id)
            return True  # 停止运行
            
        elif action == "notify":
            event_type = msg.get("event_type", "")
            data = msg.get("data", {})
            handle_notify(request_id, event_type, data)
            return False  # 继续运行
    
    # 忽略其他类型的消息
    return False


def main():
    """主循环：读取并处理消息"""
    # 主循环：读取并处理消息
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        
        try:
            msg = json.loads(line)
            should_exit = handle_message(msg)
            if should_exit:
                break
        except json.JSONDecodeError as e:
            print(f"Failed to parse message: {e} - Line: {line}", file=sys.stderr)
        except Exception as e:
            print(f"Error handling message: {e}", file=sys.stderr)


if __name__ == "__main__":
    main()

