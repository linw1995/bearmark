#[cfg(not(tarpaulin_include))]
#[allow(clippy::result_large_err)]
#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _rocket = bearmark_api::rocket().await.launch().await?;

    Ok(())
}
