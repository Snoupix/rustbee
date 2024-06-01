use bluer::Session;

#[tokio::main]
async fn main() {
    let session = Session::new().await.unwrap();
    let adapter = session.default_adapter().await.unwrap();
    println!("{}", adapter.name());
}
