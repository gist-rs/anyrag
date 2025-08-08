use anyquery::{
    providers::ai::{gemini::GeminiProvider, local::LocalAiProvider},
    PromptClientBuilder,
};
use dotenvy::dotenv;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage: {} \"<prompt>\" [table_name] [instruction] [answer_key]",
            args[0]
        );
        return Ok(());
    }

    let prompt = &args[1];
    let table_name = args.get(2).map(|s| s.as_str());
    let instruction = args.get(3).map(|s| s.as_str());
    let answer_key = args.get(4).map(|s| s.as_str());
    let ai_provider_name = env::var("AI_PROVIDER").unwrap_or_else(|_| "gemini".to_string());
    let api_url = env::var("AI_API_URL").expect("AI_API_URL environment variable not set");
    let api_key = env::var("AI_API_KEY").ok();
    let ai_model = env::var("AI_MODEL").ok();
    let project_id =
        env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID environment variable not set");

    let ai_provider = match ai_provider_name.as_str() {
        "gemini" => {
            let key = api_key.expect("AI_API_KEY is required for gemini provider");
            Box::new(GeminiProvider::new(api_url, key)?)
                as Box<dyn anyquery::providers::ai::AiProvider>
        }
        "local" => Box::new(LocalAiProvider::new(api_url, api_key, ai_model)?)
            as Box<dyn anyquery::providers::ai::AiProvider>,
        _ => return Err(format!("Unsupported AI provider: {ai_provider_name}").into()),
    };

    let client = PromptClientBuilder::new()
        .ai_provider(ai_provider)
        .bigquery_storage(project_id)
        .await?
        .build()?;

    match client
        .execute_prompt(prompt, table_name, instruction, answer_key)
        .await
    {
        Ok(result) => println!("Query Result:\n{result}"),
        Err(e) => eprintln!("Error: {e}"),
    }

    Ok(())
}
