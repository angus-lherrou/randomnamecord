use std::env;
use std::thread::sleep;
use std::time::Duration;

use serenity::framework::standard::{macros::command, ArgError::*, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use rand::seq::IteratorRandom;
use rand::thread_rng;

use behindthename::{lookup, random, session::Session, types::RateLimited::*, types::*};

use core::result::Result;  // satisfy IntelliJ's erroneous type checking

struct Name {
    first_name: String,
    last_name_result: Result<String, String>,
}

fn hyperlink(title: &str, url: &str) -> String {
    format!("[{}]({})", title, url)
}

fn first_name_url(first_name: &str) -> String {
    format!(
        "https://www.behindthename.com/name/{}",
        first_name.to_lowercase()
    )
}

fn last_name_url(last_name: &str) -> String {
    format!(
        "https://surnames.behindthename.com/name/{}",
        last_name.to_lowercase()
    )
}

fn first_name_hyperlink(first_name: &str) -> String {
    hyperlink(first_name, &first_name_url(first_name))
}

fn last_name_hyperlink(last_name: &str) -> String {
    hyperlink(last_name, &last_name_url(last_name))
}

fn _name(mut args: Args) -> Result<Name, String> {
    let key_string = env::var("BTN_API_KEY").unwrap();
    let key = key_string.as_str();
    let session = Session::new_default(key);

    let gender = match args.single::<Gender>() {
        Ok(g) => Ok(g),
        Err(Parse(_)) => Err("Could not parse gender".to_string()),
        Err(Eos) => Ok(Gender::Any),
        Err(_) => Err("some other error".to_string()),
    }?;

    sleep(Duration::from_millis(550));

    let first_name_request = random::random_with_gender(gender);
    let first_name = match session.request(first_name_request) {
        Allowed(JsonResponse::NameList(JsonNameList { names })) => {
            Ok(names.first().unwrap().to_owned())
        }
        Allowed(_) => Err("At first name request: parsing issue".into()),
        Limited(l) => Err(format!("At first name request: {:?}", l)),
        Governed(_, _) => Err("At first name request: governed".into()),
        Error(e) => Err(format!("At first name request: {}", e))
    }?;

    sleep(Duration::from_millis(550));

    let usage_request = lookup::lookup(first_name.as_str());
    let usage = match session.request(usage_request) {
        Allowed(JsonResponse::NameDetails(JsonNameDetails(details))) => details
            .into_iter()
            .next()
            .and_then(|first| first.usages.into_iter().choose(&mut thread_rng())).ok_or("No Usage".into()),
        Allowed(_) => Err("At usage request: parsing issue".into()),
        Limited(l) => Err(format!("At usage request: {:?}", l)),
        Governed(_, _) => Err("At usage request: governed".into()),
        Error(e) => Err(format!("At usage request: {}", e))
    };

    sleep(Duration::from_millis(550));

    let last_name_result = usage.and_then(|u| {
        let last_name_gender = match gender {
            Gender::Any => u.usage_gender,
            _ => gender,
        };
        let last_name_request =
            random::random_with_params(last_name_gender, Some(&*u.usage_code), Some(1), true);
        match session.request(last_name_request) {
            Allowed(JsonResponse::NameList(JsonNameList { names })) => {
                Ok(names.last().unwrap().to_owned())
            }
            Allowed(_) => Err("At last name request: parsing error".into()),
            Limited(l) => Err(format!("At last name request for usage {:?}: {:?}", u, l)),
            Governed(_, _) => Err("At last name request: governed".into()),
            Error(e) => Err(format!("At last name request: {}", e)),
        }
    });

    Ok(Name { first_name, last_name_result })
}

fn no_last_name(err: String) -> String {
    format!("

This name is a mononym. Pretend you're Cher. Or Zeus.

||{}||", err)
}

#[command]
pub async fn name(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let typing = msg.channel_id.start_typing(&ctx.http)?;

    let Name { first_name, last_name_result } = tokio::task::spawn_blocking(move || _name(args)).await??;

    let full_name = format!("{} {}", first_name.clone(), match last_name_result.clone() {
        Ok(last_name) => last_name,
        Err(error) => no_last_name(error)
    });

    typing.stop();

    if let Err(e) = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.content(full_name).embed(|e| {
                e.title("BehindTheName").field(
                    "First Name",
                    first_name_hyperlink(&first_name),
                    true,
                );
                if let Ok(last_name) = last_name_result {
                    e.field("Last Name", last_name_hyperlink(&last_name), true);
                }
                e
            })
        })
        .await
    {
        msg.channel_id.say(&ctx.http, format!("{}", &e)).await?;
        Err(e)?
    }

    Ok(())
}
