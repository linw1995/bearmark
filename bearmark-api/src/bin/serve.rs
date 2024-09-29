#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _rocket = bearmark_api::rocket().await.launch().await?;

    Ok(())
}
