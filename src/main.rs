mod binutils;
mod cmds;

#[tokio::main]
async fn main() {
    cmds::cmd_loader().await;
}
