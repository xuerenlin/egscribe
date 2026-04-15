use egscribe::config::Config;
use serde_json;

#[test]
fn test_config_backward_compatibility() {
    // 模拟旧版本的 JSON 配置（缺少新字段）
    let old_config_json = r#"{
        "show_line_no": true,
        "show_index_window": false,
        "wrap": true,
        "font_size": 14.0,
        "indent_size": 20.0,
        "dark_mode": false,
        "current_file": "test.md",
        "opend_files": ["file1.md"],
        "tree_open_state": {},
        "tree_open_state_changed": true
    }"#;

    // 尝试解析旧版本的 JSON
    match serde_json::from_str::<Config>(old_config_json) {
        Ok(config) => {
            // 验证现有字段正确解析
            assert_eq!(config.show_line_no, true);
            assert_eq!(config.show_index_window, false);
            assert_eq!(config.wrap, true);
            assert_eq!(config.font_size, 14.0);
            assert_eq!(config.indent_size, 20.0);
            assert_eq!(config.dark_mode, false);
            assert_eq!(config.current_file, "test.md");
            assert_eq!(config.opend_files, vec!["file1.md"]);
            assert_eq!(config.tree_open_state_changed, true);
            
            // 验证新字段使用默认值
            assert_eq!(config.default_charset, "UTF-8");
            assert_eq!(config.auto_detect_encoding, true);
            
            println!("✅ 向后兼容性测试通过！");
        }
        Err(e) => {
            panic!("❌ 向后兼容性测试失败: {}", e);
        }
    }
}

#[test]
fn test_config_default_values() {
    // 测试空 JSON 对象
    let empty_config_json = "{}";
    
    match serde_json::from_str::<Config>(empty_config_json) {
        Ok(config) => {
            // 验证所有字段都使用默认值
            assert_eq!(config.show_line_no, true);
            assert_eq!(config.show_index_window, true);
            assert_eq!(config.wrap, false);
            assert_eq!(config.font_size, 16.0);
            assert_eq!(config.indent_size, 16.0);
            assert_eq!(config.dark_mode, true);
            assert_eq!(config.current_file, "");
            assert_eq!(config.opend_files, Vec::<String>::new());
            assert_eq!(config.tree_open_state, std::collections::HashMap::new());
            assert_eq!(config.tree_open_state_changed, false);
            assert_eq!(config.default_charset, "UTF-8");
            assert_eq!(config.auto_detect_encoding, true);
            
            println!("✅ 默认值测试通过！");
        }
        Err(e) => {
            panic!("❌ 默认值测试失败: {}", e);
        }
    }
} 