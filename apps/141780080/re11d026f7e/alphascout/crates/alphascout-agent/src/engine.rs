use crate::aomi::bridge::execute;

pub async fn process_message(message: String) -> String {
    execute(message).await
}
