use anyquery::PromptClientBuilder;
use dotenvy::dotenv;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} \"<prompt>\" [table_name] [instruction]", args[0]);
        return Ok(());
    }

    let prompt = &args[1];
    let table_name = args.get(2).map(|s| s.as_str());
    let instruction = args.get(3).map(|s| s.as_str());
    let api_url = env::var("GEMINI_API_URL").expect("GEMINI_API_URL environment variable not set");
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");
    let project_id =
        env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID environment variable not set");

    let client = PromptClientBuilder::new()
        .gemini_url(api_url)
        .gemini_api_key(api_key)
        .bigquery_storage(project_id)
        .await?
        .build()?;

    match client.execute_prompt(prompt, table_name, instruction).await {
        Ok(result) => println!("Query Result:\n{result}"),
        Err(e) => eprintln!("Error: {e}"),
    }

    Ok(())
}
