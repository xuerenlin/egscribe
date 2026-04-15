mod common;

use common::{assert_doc, execute_action, md_ctx, set_caret_at_line_char};
use egscribe::medit::{Action, Ctx};
use std::collections::HashMap;

#[test]
fn set_expanded_text_smoke_creates_sub_editor() {
    let mut ctx = md_ctx("line0");
    set_caret_at_line_char(&mut ctx, 0, 0);
    let mut p = HashMap::new();
    p.insert("line_no".to_string(), serde_json::json!(0));
    p.insert("expanded_text".to_string(), serde_json::json!("inner md"));
    let action = Action::from_command("set_expanded_text", &p).expect("valid params");
    execute_action(&mut ctx, &action);
    assert_doc(&ctx, "line0");
    assert!(
        ctx.get_line(0).unwrap().expanded_text_id.is_some(),
        "expanded_text_id after set"
    );
    assert!(ctx.expanded_ctx().has_ctx());
}

fn set_expanded_on_line0(ctx: &mut Ctx) {
    let mut p = HashMap::new();
    p.insert("line_no".to_string(), serde_json::json!(0));
    p.insert("expanded_text".to_string(), serde_json::json!("inner"));
    let action = Action::from_command("set_expanded_text", &p).expect("valid params");
    execute_action(ctx, &action);
}

#[test]
fn set_expanded_text_clear_when_expanded_text_param_omitted() {
    let mut ctx = md_ctx("row");
    set_caret_at_line_char(&mut ctx, 0, 0);
    set_expanded_on_line0(&mut ctx);
    assert!(ctx.expanded_ctx().has_ctx());

    let mut clear = HashMap::new();
    clear.insert("line_no".to_string(), serde_json::json!(0));
    let action = Action::from_command("set_expanded_text", &clear).expect("clear params");
    execute_action(&mut ctx, &action);

    assert!(ctx.get_line(0).unwrap().expanded_text_id.is_none());
    assert!(!ctx.expanded_ctx().has_ctx());
}

#[test]
fn set_expanded_text_clear_when_expanded_text_json_null() {
    let mut ctx = md_ctx("row");
    set_caret_at_line_char(&mut ctx, 0, 0);
    set_expanded_on_line0(&mut ctx);

    let mut clear = HashMap::new();
    clear.insert("line_no".to_string(), serde_json::json!(0));
    clear.insert("expanded_text".to_string(), serde_json::Value::Null);
    let action = Action::from_command("set_expanded_text", &clear).expect("null param");
    execute_action(&mut ctx, &action);

    assert!(ctx.get_line(0).unwrap().expanded_text_id.is_none());
    assert!(!ctx.expanded_ctx().has_ctx());
}

#[test]
fn set_expanded_text_clear_when_expanded_text_empty_string() {
    let mut ctx = md_ctx("row");
    set_caret_at_line_char(&mut ctx, 0, 0);
    set_expanded_on_line0(&mut ctx);

    let mut clear = HashMap::new();
    clear.insert("line_no".to_string(), serde_json::json!(0));
    clear.insert("expanded_text".to_string(), serde_json::json!(""));
    let action = Action::from_command("set_expanded_text", &clear).expect("empty string");
    execute_action(&mut ctx, &action);

    assert!(ctx.get_line(0).unwrap().expanded_text_id.is_none());
    assert!(!ctx.expanded_ctx().has_ctx());
}
