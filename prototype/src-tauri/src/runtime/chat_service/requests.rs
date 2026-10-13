use crate::runtime::{Reply, Runtime};
use serde_json::{json, Value};

impl Runtime {
    pub fn reply(&self, body: &Value) -> Reply {
        let id = &body["id"];
        let request = self
            .store
            .lock()
            .unwrap()
            .chats
            .iter()
            .flat_map(|chat| &chat.requests)
            .find(|request| &request["id"] == id)
            .cloned()
            .ok_or("请求已结束或不存在")?;
        let method = request["method"].as_str().unwrap_or("");
        let result = match method {
            "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => {
                let decision = validated_decision(body)?;
                json!({"decision": decision})
            }
            "applyPatchApproval" | "execCommandApproval" => {
                let decision = validated_decision(body)?;
                json!({"decision": if decision == "accept" { "approved" } else { "abort" }})
            }
            "item/permissions/requestApproval" => {
                let decision = validated_decision(body)?;
                let permissions = if decision == "accept" {
                    request["params"]["permissions"].clone()
                } else {
                    json!({})
                };
                json!({"permissions":permissions,"scope":"turn"})
            }
            "item/tool/requestUserInput" => {
                let decision = validated_decision(body)?;
                let answers = if decision == "accept" {
                    body["answers"].as_object().cloned().unwrap_or_default()
                } else {
                    serde_json::Map::new()
                };
                json!({"answers":answers})
            }
            "mcpServer/elicitation/request" => {
                let decision = validated_decision(body)?;
                elicitation_result(decision, body)
            }
            _ => return Err("不支持此类请求".into()),
        };
        self.write(&json!({"id":id,"result":result}))?;
        for chat in &mut self.store.lock().unwrap().chats {
            chat.requests.retain(|request| &request["id"] != id);
        }
        self.publish_event("chat-ui/request/replied", &json!({"requestId": id}));
        Ok(json!({"ok":true}))
    }
}

fn validated_decision(body: &Value) -> Result<&str, String> {
    let decision = body["decision"].as_str().unwrap_or("decline");
    if ["accept", "decline", "cancel"].contains(&decision) {
        Ok(decision)
    } else {
        Err("无效的决定".into())
    }
}

fn elicitation_result(decision: &str, body: &Value) -> Value {
    let action = match decision {
        "accept" => "accept",
        "cancel" => "cancel",
        _ => "decline",
    };
    let content = if decision == "accept" {
        body.get("content")
            .or_else(|| body.get("answers"))
            .cloned()
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    json!({"action":action,"content":content,"_meta":Value::Null})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_c09_decisions_are_explicit_and_safe_by_default() {
        assert_eq!(
            validated_decision(&json!({"decision":"accept"})),
            Ok("accept")
        );
        assert_eq!(validated_decision(&json!({})), Ok("decline"));
        assert!(validated_decision(&json!({"decision":"grant-all"})).is_err());
    }

    #[test]
    fn case_c09_elicitation_cancel_is_not_encoded_as_decline() {
        assert_eq!(
            elicitation_result("cancel", &json!({"content":{"secret":"no"}})),
            json!({"action":"cancel","content":null,"_meta":null})
        );
        assert_eq!(
            elicitation_result("accept", &json!({"content":{"choice":"yes"}})),
            json!({"action":"accept","content":{"choice":"yes"},"_meta":null})
        );
    }
}
