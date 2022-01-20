use std::env;
use std::thread::sleep;
use std::time::Duration;

use serenity::framework::standard::{macros::command, ArgError::*, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use rand::seq::IteratorRandom;
use rand::thread_rng;

use behindthename::{lookup, random, session::Session, types::RateLimited::*, types::*};

#[command]
pub async fn name(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let name: core::result::Result<String, &str> = tokio::task::spawn_blocking(move || {
        let key_string = env::var("BTN_API_KEY").unwrap();
        let key = key_string.as_str();
        let session = Session::new_default(key);

        let gender = match args.single::<Gender>() {
            Ok(g) => Ok(g),
            Err(Parse(_)) => Err("Could not parse gender"),
            Err(Eos) => Ok(Gender::Any),
            Err(_) => Err("some other error"),
        }?;
        let first_name_request = random::random_with_gender(gender);
        let first_name = match session.request(first_name_request) {
            Allowed(JsonResponse::NameList(JsonNameList { names })) => {
                Ok(names.first().unwrap().to_owned())
            }
            _ => Err("request failed: first name"),
        }?;

        sleep(Duration::from_millis(550));

        let usage_request = lookup::lookup(first_name.as_str());
        let usage = match session.request(usage_request) {
            Allowed(JsonResponse::NameDetails(JsonNameDetails(details))) => details
                .into_iter()
                .next()
                .and_then(|first| first.usages.into_iter().choose(&mut thread_rng())),
            _ => None,
        };

        sleep(Duration::from_millis(550));

        let last_name_option = usage
            .and_then(|u| {
                let last_name_gender = match gender {
                    Gender::Any => u.usage_gender,
                    _ => gender,
                };
                let last_name_request = random::random_with_params(
                    last_name_gender,
                    Some(&*u.usage_code),
                    Some(1),
                    true,
                );
                // match session.request(last_name_request) {
                //     Allowed(JsonResponse::NameList(JsonNameList { names })) => {
                //         Some(names.last().unwrap().to_owned())
                //     }
                //     Allowed(_) => Some("DEBUG: other allowed".into()),
                //     Governed(_, _) => Some("DEBUG: governed".into()),
                //     Limited(_) => Some("DEBUG: limited".into()),
                //     Error(_) => Some("DEBUG: error".into()),
                // }
                if let Allowed(JsonResponse::NameList(JsonNameList { names })) =
                    session.request(last_name_request)
                {
                    Some(names.last().unwrap().to_owned())
                } else {
                    None
                }
            });
            // .or(Some("DEBUG: no usage response".into()));
        Ok(last_name_option.map_or(first_name.clone(), |last_name| {
            format!("{} {}", first_name, last_name)
        }))
    })
    .await?;

    msg.channel_id.say(&ctx.http, name?).await?;

    sleep(Duration::from_millis(550));

    Ok(())
}
