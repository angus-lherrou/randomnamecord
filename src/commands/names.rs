use std::env;
use std::thread::sleep;
use std::time::Duration;

use behindthename::{lookup, random, session::Session, types::RateLimited::*, types::*};
use rand::seq::SliceRandom;
use rand::thread_rng;
use reqwest::Client;
use serenity::framework::standard::{
    macros::command, ArgError::*, Args, CommandError, CommandResult,
};
use serenity::{builder::CreateMessage, model::prelude::*, prelude::*};
use tracing::error;

use core::result::Result; // satisfy IntelliJ's erroneous type checking

const VERSION: &str = env!("CARGO_PKG_VERSION");

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
        Error(e) => Err(format!("At first name request: {}", e)),
    }?;

    sleep(Duration::from_millis(550));

    let usage_request = lookup::lookup(first_name.as_str());

    let possible_usages = match session.request(usage_request) {
        Allowed(JsonResponse::NameDetails(JsonNameDetails(details))) => Ok(
            details
                .into_iter()
                .map(|item| item.usages)
                .flatten()
                .collect::<Vec<_>>()
        ),
        Allowed(_) => Err("At usage request: parsing issue".into()),
        Limited(l) => Err(format!("At usage request: {:?}", l)),
        Governed(_, _) => Err("At usage request: governed".into()),
        Error(e) => Err(format!("At usage request: {}", e)),
    };

    let possible_usages_shuffled = possible_usages.and_then(
        |mut usages| {
            usages.shuffle(&mut thread_rng());
            Ok(usages)
        }
    );

    let last_name_result = possible_usages_shuffled.and_then(|usages| {
        let mut errs_acc = vec!();
        let mut result = Err("No Usage".into());
        for usage in usages {
            sleep(Duration::from_millis(550));

            let last_name_gender = match gender {
                Gender::Any => usage.usage_gender,
                _ => gender,
            };
            let last_name_request =
                random::random_with_params(last_name_gender, Some(&*usage.usage_code), Some(1), true);
            let matched = match session.request(last_name_request) {
                Allowed(JsonResponse::NameList(JsonNameList { names })) => {
                    Ok(Ok(names.last().unwrap().to_owned()))
                }
                Allowed(_) => Err("At last name request: parsing error".into()),
                Limited(NotAvailable { error_code: 60, .. }) => { Ok(Err(())) }
                Limited(l) => {
                    Err(format!("At last name request for usage {:?}: {:?}", usage, l))
                }
                Governed(_, _) => Err("At last name request: governed".into()),
                Error(e) => Err(format!("At last name request: {}", e)),
            };
            match matched {
                Ok(Ok(name)) => {
                    result = Ok(name);
                    break
                }
                Ok(Err(())) => continue,
                Err(e) => {
                    errs_acc.push(e)
                }
            }
        };
        match result {
            Ok(r) => {
                if !errs_acc.is_empty() {
                    println!("SOME_ERRORS");
                    println!("{}", errs_acc.join("\n"));
                }
                Ok(r)
            }
            Err(e) => match errs_acc.len() {
                0 => Err(e),
                _ => Err(errs_acc.join("\n"))
            }
        }
    });

    Ok(Name {
        first_name,
        last_name_result,
    })
}

fn no_last_name(err: String) -> String {
    format!("

This name is a mononym. Pretend you're Cher. Or Zeus.

||{}||", err)
}

#[command]
pub async fn name(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let typing = msg.channel_id.start_typing(&ctx.http)?;

    let Name {
        first_name,
        last_name_result,
    } = tokio::task::spawn_blocking(move || _name(args)).await??;

    let full_name = format!(
        "{} {}",
        first_name.clone(),
        match last_name_result.clone() {
            Ok(last_name) => last_name,
            Err(error) => no_last_name(error),
        }
    );

    let _ = typing.stop();

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

pub async fn _about<'a>(
    ctx: &'a Context,
    msg: &'a Message,
    args: Args,
) -> Result<CreateMessage<'a>, CommandError> {
    let nick_opt = msg.author_nick(&ctx.http).await;
    let raw_args = args.raw().collect::<Vec<&str>>();
    let names = if !raw_args.is_empty() {
        raw_args
    } else if let Some(nick) = &nick_opt {
        nick.split_whitespace().collect()
    } else {
        vec![]
    };

    let clx = Client::new();

    match names.len() {
        0 => {
            let mut m = CreateMessage::default();
            Ok(m.content("No name found").to_owned())
        }
        1 => {
            let first = names.first().unwrap();
            let url = first_name_url(first);
            let url_opt = clx
                .head(&url)
                .send()
                .await
                .map_err(|e| format!("1: {:?}, {:?}, {}", &e, e.url(), &url))?
                .status()
                .is_success()
                .then(|| url);
            let mut m = CreateMessage::default();
            Ok(match url_opt {
                Some(url) => m.content(&first).embed(|e| {
                    e.title("BehindTheName")
                        .field("First Name", hyperlink(first, &url), true)
                }),
                None => m.content(format!("Name {} not found.", first)),
            }
            .to_owned())
        }
        _ => {
            let last = names.last().unwrap();
            let mut urls: Vec<Option<String>> = vec![];
            for name in (&names[..names.len() - 1]).iter() {
                let url = first_name_url(name);
                let status = clx
                    .head(&url)
                    .send()
                    .await
                    .map_err(|e| format!("2: {:?}, {:?}, {}", &e, e.url(), &url))?
                    .status();
                urls.push(status.is_success().then(|| url));
            }

            let last_url = last_name_url(last);
            let last_url_opt = clx
                .head(&last_url)
                .send()
                .await
                .map_err(|e| format!("3: {:?}, {:?}, {}", &e, e.url(), &last_url))?
                .status()
                .is_success()
                .then(|| last_url)
                .or(async {
                    let last_first_url = first_name_url(last);
                    clx.head(&last_first_url)
                        .send()
                        .await
                        .ok()?
                        .status()
                        .is_success()
                        .then(|| last_first_url)
                }
                .await);

            urls.push(last_url_opt);

            let names_and_urls = (&names).iter().zip(&urls);

            let mut final_urls: Vec<(&str, &String)> = names_and_urls
                .filter_map(|(n, u)| u.as_ref().map(|v| (*n, v)))
                .collect();

            match final_urls.len() {
                0 => {
                    let mut m = CreateMessage::default();
                    Ok(m.content("No names found.").to_owned())
                }
                1 => {
                    let mut m = CreateMessage::default();
                    let (name, url) = final_urls.pop().unwrap();
                    Ok(m.content(names.join(" "))
                        .embed(|e| {
                            e.title("BehindTheName")
                                .field("First Name", hyperlink(name, url), true)
                        })
                        .to_owned())
                }
                _ => {
                    let mut m = CreateMessage::default();
                    Ok(m.content(names.join(" "))
                        .embed(|e| {
                            let (last_name, last_url) = final_urls.pop().unwrap();

                            let mut url_iter = final_urls.into_iter();
                            let (first_name, first_url) = url_iter.next().unwrap();
                            e.title("BehindTheName").field(
                                "First Name",
                                hyperlink(first_name, first_url),
                                true,
                            );

                            for (name, url) in url_iter {
                                e.field("Middle Name", hyperlink(name, url), true);
                            }

                            e.field("Last Name", hyperlink(last_name, last_url), true);
                            e
                        })
                        .to_owned())
                }
            }
        }
    }
}

#[command]
pub async fn about(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let typing = msg.channel_id.start_typing(&ctx.http)?;
    let r = _about(ctx, msg, args).await;
    let _ = typing.stop();
    match r {
        Ok(mut m) => {
            msg.channel_id.send_message(&ctx.http, |_| &mut m).await?;
            Ok(())
        }
        Err(e) => {
            error!("{:?}", e);
            Ok(())
        }
    }
}

#[command]
pub async fn help(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let typing = msg.channel_id.start_typing(&ctx.http)?;
    let mut m = CreateMessage::default();
    m.content(format!(
        "\
randomnamecord version {}
Commands:
  ~name [m|f|mf|u]: generate a name, optionally with a specific gender.
  ~about [name]: find the Behind The Name info pages for the provided name, \
or for your nickname if no name is provided.
  ~help: print this help page.", VERSION));
    let _ = typing.stop();
    msg.channel_id.send_message(&ctx.http, |_| &mut m).await?;
    Ok(())
}
