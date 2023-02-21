use std::env;
use std::thread::sleep;
use std::time::Duration;

use behindthename::{lookup, random, session::Session, types::RateLimited::*, types::*};
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::thread_rng;
use reqwest::Client;
use unicode_normalization::UnicodeNormalization;

use poise::serenity_prelude::CacheHttp;

use crate::resources::maps::*;
use crate::resources::types::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct Name {
    first_name: String,
    last_name_result: Result<String, String>,
}

fn hyperlink(title: &str, url: &str) -> String {
    format!("[{}]({})", title, url)
}

fn lower_normalize(name: &str) -> String {
    let nfc_name = name.to_lowercase().nfc().to_string();

    NORM_AC.replace_all(&nfc_name, &NORM_CODES)
}

fn first_name_url(first_name: &str) -> String {
    format!(
        "https://www.behindthename.com/name/{}",
        lower_normalize(first_name)
    )
}

fn last_name_url(last_name: &str) -> String {
    format!(
        "https://surnames.behindthename.com/name/{}",
        lower_normalize(last_name)
    )
}

fn first_name_hyperlink(first_name: &str) -> String {
    hyperlink(first_name, &first_name_url(first_name))
}

fn last_name_hyperlink(last_name: &str) -> String {
    hyperlink(last_name, &last_name_url(last_name))
}

fn _surname(session: Session, gender: Gender, first_name: String) -> Result<Name, String> {
    sleep(Duration::from_millis(550));

    let usage_request = lookup::lookup(first_name.as_str());

    let possible_usages = match session.request(usage_request) {
        Allowed(JsonResponse::NameDetails(JsonNameDetails(details))) => Ok(details
            .into_iter()
            .flat_map(|item| item.usages)
            .unique()
            .collect::<Vec<_>>()),
        Allowed(_) => Err("At usage request: parsing issue".into()),
        Failed(e) => Err(format!("At usage request: {:?}", e)),
        Governed(_, _) => Err("At usage request: governed".into()),
        ReqwestError(e) => Err(format!("At usage request: {}", e)),
    };

    let possible_usages_shuffled = possible_usages.map(|mut usages| {
        usages.shuffle(&mut thread_rng());
        usages
    });

    let mut usage_augment = possible_usages_shuffled
        .as_ref()
        .map(|usages| {
            usages
                .iter()
                .filter(|usage| USAGE_REGEX.is_match(&usage.usage_code))
                .map(|usage| {
                    let mut new_usage_code = None;
                    for (pat, repl) in USAGE_MAP.iter() {
                        if pat.is_match(&usage.usage_code) {
                            new_usage_code =
                                Some(pat.replace(&usage.usage_code, repl.to_string()).to_string());
                            break;
                        }
                    }
                    Usage {
                        usage_code: new_usage_code.unwrap(),
                        ..usage.clone()
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let possible_usages_augmented = possible_usages_shuffled.map(|mut usages| {
        usages.append(&mut usage_augment);
        usages
    });

    let last_name_result = possible_usages_augmented.and_then(|usages| {
        let mut errs_acc = vec![];
        let mut result = Err(format!("{} Usages", usages.len()));
        for usage in usages {
            sleep(Duration::from_millis(550));

            let last_name_gender = match gender {
                Gender::Any => usage.usage_gender,
                _ => gender,
            };
            let last_name_request = random::random_with_params(
                last_name_gender,
                Some(&*usage.usage_code),
                Some(1),
                true,
            );
            let matched = match session.request(last_name_request) {
                Allowed(JsonResponse::NameList(JsonNameList { names })) => {
                    Ok(names.last().unwrap().to_owned())
                }
                Allowed(_) => Err("At last name request: parsing error".into()),
                Failed(e) => Err(format!(
                    "At last name request for usage {:?}: {:?}",
                    usage, e
                )),
                Governed(_, _) => Err("At last name request: governed".into()),
                ReqwestError(e) => Err(format!("At last name request: {}", e)),
            };
            match matched {
                Ok(name) => {
                    result = Ok(name);
                    break;
                }
                Err(e) => errs_acc.push(e),
            }
        }
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
                _ => Err(errs_acc.join("\n")),
            },
        }
    });

    Ok(Name {
        first_name,
        last_name_result,
    })
}

fn _name(gender_opt: Option<Gender>, mode_opt: Option<GenMode>) -> Result<Name, String> {
    let key_string = env::var("BTN_API_KEY").unwrap();
    let key = key_string.as_str();
    let session = Session::new_default(key);

    let gender = gender_opt.unwrap_or(Gender::Any);

    let mode = mode_opt.unwrap_or(GenMode::Coherent);

    sleep(Duration::from_millis(550));

    match mode {
        GenMode::Coherent => {
            let first_name_request = random::random_with_gender(gender);
            let first_name = match session.request(first_name_request) {
                Allowed(JsonResponse::NameList(JsonNameList { names })) => {
                    Ok(names.first().unwrap().to_owned())
                }
                Allowed(_) => Err("At first name request: parsing issue".into()),
                Failed(e) => Err(format!("At first name request: {:?}", e)),
                Governed(_, _) => Err("At first name request: governed".into()),
                ReqwestError(e) => Err(format!("At first name request: {}", e)),
            }?;

            _surname(session, gender, first_name)
        }
        GenMode::Chaotic => {
            let name_request = random::random_with_params(gender, None, Some(1), true);
            let name_vec = match session.request(name_request) {
                Allowed(JsonResponse::NameList(JsonNameList { names })) => Ok(names),
                Allowed(_) => Err("At first name request: parsing issue".into()),
                Failed(e) => Err(format!("At first name request: {:?}", e)),
                Governed(_, _) => Err("At first name request: governed".into()),
                ReqwestError(e) => Err(format!("At first name request: {}", e)),
            }?;
            match name_vec.len() {
                1..=2 => Ok(Name {
                    first_name: name_vec.first().unwrap().to_owned(),
                    last_name_result: name_vec
                        .get(1)
                        .ok_or_else(|| "At last name request: none found?".into())
                        .cloned(),
                }),
                0 => Err("No name fetched".into()),
                _ => Err("Too many names fetched".into()),
            }
        }
    }
}

fn _dbg_name(first_name: String, gender_opt: Option<Gender>) -> Result<Name, String> {
    let key_string = env::var("BTN_API_KEY").unwrap();
    let key = key_string.as_str();
    let session = Session::new_default(key);

    let gender = gender_opt.unwrap_or(Gender::Any);

    _surname(session, gender, first_name)
}

fn no_last_name(err: String) -> String {
    format!(
        "

This name is a mononym. Pretend you're Cher. Or Zeus.

||{}||",
        err
    )
}

fn an_error_occurred(err: String) -> String {
    format!(
        "An error occurred.

||{}||",
        err
    )
}

/// Generate a random name, optionally with a specific gender and mode.
///
/// Generate a random name, optionally with a specific gender and mode.
///
/// The gender options are m, f, u. Passing "u" will generate \
/// an androgynous name; leaving gender blank will generate \
/// a name of any gender.
///
/// The generation modes are as follows:
///
///  * coherent: will attempt to generate a name with a coherent given name and surname.
///  * chaotic: will generate a given name and surname completely at random.
#[poise::command(prefix_command, slash_command, broadcast_typing)]
pub(crate) async fn name(
    ctx: Context<'_>,
    #[description = "Gender of name, optional: m|f|u"] gender: Option<Gender>,
    #[description = "Generation mode, optional: coherent|chaotic"] mode: Option<GenMode>,
) -> Result<(), Error> {
    let working_msg = ctx.say("Working...").await?;

    let name = tokio::task::spawn_blocking(move || _name(gender, mode)).await?;

    match name {
        Ok(Name {
            first_name,
            last_name_result,
        }) => {
            let full_name = format!(
                "{} {}",
                first_name.clone(),
                match last_name_result.clone() {
                    Ok(last_name) => last_name,
                    Err(error) => no_last_name(error),
                }
            );
            working_msg
                .edit(ctx, |m| {
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
                .await?;
        }
        Err(e) => {
            ctx.say(an_error_occurred(e.clone())).await?;
            Err(e)?
        }
    }

    Ok(())
}

/// Generate a random name.
///
/// Debug command; please ignore.
#[poise::command(prefix_command, slash_command, broadcast_typing, hide_in_help)]
pub(crate) async fn debug_name(
    ctx: Context<'_>,
    #[description = "First name"] first_name: String,
    #[description = "Gender of name"] gender: Option<Gender>,
) -> Result<(), Error> {
    let working_msg = ctx.say("Working...").await?;

    let name = tokio::task::spawn_blocking(move || _dbg_name(first_name, gender)).await?;

    match name {
        Ok(Name {
            first_name,
            last_name_result,
        }) => {
            let full_name = format!(
                "{} {}",
                first_name.clone(),
                match last_name_result.clone() {
                    Ok(last_name) => last_name,
                    Err(error) => no_last_name(error),
                }
            );
            working_msg
                .edit(ctx, |m| {
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
                .await?;
        }
        Err(e) => {
            ctx.say(an_error_occurred(e.clone())).await?;
            Err(e)?
        }
    }

    Ok(())
}

async fn get_name_vector(ctx: Context<'_>, name: Option<String>) -> Vec<String> {
    let nick_opt = ctx
        .author()
        .nick_in(ctx.http(), ctx.guild_id().unwrap())
        .await;

    if let Some(n) = &name {
        n.split_whitespace().map(|s| s.to_owned()).collect()
    } else if let Some(nick) = &nick_opt {
        nick.split_whitespace().map(|s| s.to_owned()).collect()
    } else {
        vec![]
    }
}

struct MessageContent {
    content: String,
    embed: Option<MessageEmbed>,
}

struct MessageEmbed {
    title: String,
    fields: Vec<(String, String, bool)>,
}

async fn _about<'att>(names: Vec<String>) -> Result<MessageContent, Error> {
    let clt = Client::new();

    Ok(match names.len() {
        0 => MessageContent {
            content: "No name found".into(),
            embed: None,
        },
        1 => {
            let first = names.first().unwrap();
            let url = first_name_url(first);
            let url_opt = clt
                .head(&url)
                .send()
                .await
                .map_err(|e| format!("1: {:?}, {:?}, {}", &e, e.url(), &url))?
                .status()
                .is_success()
                .then_some(url);
            match url_opt {
                Some(url) => MessageContent {
                    content: first.into(),
                    embed: Some(MessageEmbed {
                        title: "BehindTheName".into(),
                        fields: vec![("First Name".into(), hyperlink(first, &url), true)],
                    }),
                },
                None => MessageContent {
                    content: format!("Name {} not found.", first),
                    embed: None,
                },
            }
        }
        _ => {
            let last = names.last().unwrap();
            let mut urls: Vec<Option<String>> = vec![];
            for name in names[..names.len() - 1].iter() {
                let url = first_name_url(name);
                let status = clt
                    .head(&url)
                    .send()
                    .await
                    .map_err(|e| format!("2: {:?}, {:?}, {}", &e, e.url(), &url))?
                    .status();
                urls.push(status.is_success().then_some(url));
            }

            let last_url = last_name_url(last);
            let last_url_opt = clt
                .head(&last_url)
                .send()
                .await
                .map_err(|e| format!("3: {:?}, {:?}, {}", &e, e.url(), &last_url))?
                .status()
                .is_success()
                .then_some(last_url)
                .or(async {
                    let last_first_url = first_name_url(last);
                    clt.head(&last_first_url)
                        .send()
                        .await
                        .ok()?
                        .status()
                        .is_success()
                        .then_some(last_first_url)
                }
                .await);

            urls.push(last_url_opt);

            let names_and_urls = names.iter().zip(&urls);

            let mut final_urls: Vec<(&String, &String)> = names_and_urls
                .filter_map(|(n, u)| u.as_ref().map(|v| (n, v)))
                .collect();

            match final_urls.len() {
                0 => MessageContent {
                    content: "No names found.".into(),
                    embed: None,
                },
                1 => {
                    let (name, url) = final_urls.pop().unwrap();
                    MessageContent {
                        content: names.join(" "),
                        embed: Some(MessageEmbed {
                            title: "BehindTheName".into(),
                            fields: vec![("First Name".into(), hyperlink(name, url), true)],
                        }),
                    }
                }
                _ => {
                    let (last_name, last_url) = final_urls.pop().unwrap();
                    let mut url_iter = final_urls.into_iter();
                    let (first_name, first_url) = url_iter.next().unwrap();

                    let mut fields = vec![];

                    fields.push(("First Name".into(), hyperlink(first_name, first_url), true));

                    for (name, url) in url_iter {
                        fields.push(("Middle Name".into(), hyperlink(name, url), true));
                    }

                    fields.push(("Last Name".into(), hyperlink(last_name, last_url), true));
                    MessageContent {
                        content: names.join(" "),
                        embed: Some(MessageEmbed {
                            title: "BehindTheName".into(),
                            fields,
                        }),
                    }
                }
            }
        }
    })
}

/// Get details about your nickname or a specific name.
///
/// Get details about your nickname or a specific name.
///
/// With no arguments, this will attempt to look up your \
/// Discord nickname on BehindTheName.
///
/// With an argument, this will attempt to look up the name \
/// you passed on BehindTheName.
///
/// For first or last names that contain spaces, this is \
/// currently slightly broken. You can get around this \
/// by replacing the spaces with "00", e.g.:
///
/// `/about_name Mary00Ann Van00Buren`
#[poise::command(prefix_command, slash_command, broadcast_typing, ephemeral)]
pub(crate) async fn about_name(
    ctx: Context<'_>,
    #[description = "Specific name"] name: Option<String>,
) -> Result<(), Error> {
    let working_msg = ctx.say("Working...").await?;

    let name_vector = get_name_vector(ctx, name).await;

    let message_content = _about(name_vector).await?;

    working_msg
        .edit(ctx, |m| {
            m.content(message_content.content);
            if let Some(MessageEmbed {
                title: t,
                fields: f,
            }) = message_content.embed
            {
                m.embed(|e| {
                    e.title(t);
                    e.fields(f);
                    e
                });
            }
            m
        })
        .await?;

    Ok(())
}

/// Show this menu
#[poise::command(prefix_command, track_edits, slash_command, ephemeral)]
pub(crate) async fn help_rnc(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<(), Error> {
    let extra_text = format!(
        "\
randomnamecord version {}

Type ~help_rnc command or /help_rnc command for more info on a command.
You can edit your message to the bot and the bot will edit its response.",
        VERSION
    );
    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: &extra_text,
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}
