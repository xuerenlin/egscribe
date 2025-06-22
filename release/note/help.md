# egscribe使用说明

egscribe是一款使用Rust+egui编写的笔记软件，它能够使用markdown语法轻松进行日常的笔记记录，同时也是一款轻量的文本编辑器。

---

# 1、支持的Markdown语法
## 1.1、标题

使用# 开头，支持六级标题：#、##、###、####、#####、######，例如：

\# 一级标题

\## 二级标题

\### 三级标题

\#### 四级标题

\##### 五级标题

\###### 六级标题

## 1.2、文本样式

这是普通文本

这是**粗体**，使用** **括起来的文本

这是*斜体*，使用* *括起来的文本

这是~~删除线~~，使用~~ ~~括起来的文本

这是 _下划线_ ，使用 _ _ 括起来的文本

这是 _*~~**嵌套样式**~~*_ ，包含所有样式

## 1.3、无序列表

使用 - 开头的无序列表，例如：

- 这是无序列表第一行
- 其他需要说明的条目
- 还有其他的事情

## 1.4、TODO列表

使用 - [ ] 开头的无序列表，当标记为已完成后，文本会使用删除线样式，例如：

- [x] 这是待做事项一
- [ ] 明天有个重要的约会
- [x] 小朋友的生日记得订蛋糕
- [ ] 其他等等

## 1.5、分割线

使用---表示分割线，例如：

---

可以上下选择整行进行删除分割线。

## 1.6、表格

1）markdown的表格格式，例如：

\|表头1|表头2|表头3|

\|--|--|--|

\|单元内容|单元内容|单元内容|

2）当按上述编辑文本后，会自动转换为表格：

|表头1|表头2|表头3|
|--|--|--|
|单元内容|单元内容|单元内容|
|其他|||


3）单击选择表格，会出“插入列按钮”，如下图的箭头按钮可以进行插入列、插入行操作。

![notitle](image_0197986a-ddb3-73c0-9e48-9fcb803f71ab.png)

4）如果要删除某行或者某列，可以全选某行或者某列，然后删除文本即可删除整行或者整列。

5）当在某个单元格内按“回车”时，会在下面新增一行。如果想再表格最下面新增普通文本，请按“Ctrl+回车”。

## 1.7、链接

- 支持**普通链接**，例如：[普通链接](http://egscribe.com.cn)，TODO：打开链接。
- 支持**文件双链**，例如：[[help.01]]，点击右上角图标可以直接打开help.01比较文件。
- 支持**图片链接**，例如：![notitle](image_01979879-b8ed-7230-8ef0-3661f00e3bda.png)，可以截图然后按“Ctrl+v”直接复制图片，复制图片会自动创建图片链接，图片文件会被保存到"note/images"目录中。

## 1.8、Blockquote，有待完善

使用>开头的文本，例如：

>Test1 
>Test2
>Test3

## 1.9、代码块

代码块支持两种样式：

1）使用```括起来的文本

2）以Tab开头的文本

如果按“回车”只是会在代码块内编辑，如果想在代码块最后面新增普通文本，请使用“Ctrl+回车”。

```Rust
fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();
    let mut file = String::new();
    if args.len() > 1 {
        file = args[1].clone();
    }

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug` or $env:RUST_LOG="debug" in windows).
    let icon = eframe::icon_data::from_png_bytes(&include_bytes!("../fonts/egscribe.png")[..]).unwrap();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(icon)
            .with_inner_size([1240.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egscribe",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MyApp::new(cc, file)))
        }),
    )
}
```


# 2、笔记管理

- 按Esc可以在侧边栏显示/隐藏笔记管理窗口

![notitle](image_01979884-aeed-7260-a97c-1616ad591819.png)

- 可以将经常需要编辑的笔记固定到工具栏中

![notitle](image_01979886-7b91-7d71-ac77-9d7cd7156b40.png)

- 将鼠标移动到某个笔记文件中，执行：新建子文件、重命名、删除 操作
- 同时，可以直接在一个笔记中使用[[help_test_newfile]]双链文件，点击右上角链接按钮即可新建并跳转到新文件。

# 3、样式

继承egui，支持Dark和Light两种样式，Dark样式如下图：

![notitle](image_0197988c-2264-74c2-bc57-006e9a4dab6c.png)

# 4、扩展功能
## 4.1 密码记录

[我的邮箱密码](passwd:123456 "123456!@#$%^") 点击链接按钮可以使用123456给“123456!@#$%^”加密。只需要修改passwd:******中的星号为正确密码，即可自动解密。

## 4.2 待补充