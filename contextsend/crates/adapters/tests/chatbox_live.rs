//! 真机集成测试（默认忽略）：对运行中的 ChatBox（带 --remote-debugging-port=9222）
//! 实际导入一条会话。运行：
//!   cargo test -p cs-adapters --test chatbox_live -- --ignored --nocapture
use cs_core::{ChatMessage, Conversation};

#[tokio::test]
#[ignore]
async fn live_import_to_chatbox() {
    let mut c = Conversation::new();
    c.title = Some("ContextSend 注入测试".into());
    c.messages
        .push(ChatMessage::user("这条会话由 ContextSend 经 CDP 注入"));
    c.messages
        .push(ChatMessage::assistant("如果你在侧栏看到我，说明导入链路打通了"));

    let id = cs_adapters::import_to_chatbox(&c)
        .await
        .expect("import failed");
    println!("IMPORTED_SESSION_ID={id}");
    assert!(!id.is_empty());
}

#[tokio::test]
#[ignore]
async fn live_list_chatbox_conversations() {
    let convos = cs_adapters::list_chatbox_conversations()
        .await
        .expect("read failed");
    println!("READ_SESSION_COUNT={}", convos.len());
    for c in convos.iter().take(8) {
        println!(
            "  - title={:?} msgs={} first={:?}",
            c.title,
            c.messages.len(),
            c.messages.first().map(|m| {
                let t = m.text();
                t.chars().take(20).collect::<String>()
            })
        );
    }
    assert!(!convos.is_empty(), "应至少读到一条会话");
}
