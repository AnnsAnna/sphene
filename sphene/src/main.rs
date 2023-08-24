use std::env;

extern crate dotenv;

use dotenv::dotenv;

use serenity::async_trait;
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::Interaction;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::model::application::interaction::InteractionType;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use thorium::twitter::UrlType;
use thorium::twitter::convert_url;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, msg: Message) {
        if !thorium::twitter::is_twitter_url(msg.content.as_str()) {
            return;
        }

        let response = MessageBuilder::new()
            .mention(&msg.author)
            .push(": ")
            .push(thorium::twitter::convert_url_lazy(msg.content.clone(), UrlType::Vxtwitter).await)
            .build();

        if let Err(why) = msg
            .channel_id
            .send_message(&context.http, |m| {
                m.allowed_mentions(|am| am.empty_parse()).content(response);
                if msg.referenced_message.is_some() {
                    m.reference_message(msg.message_reference.clone().unwrap());
                }
                m.components(|f| {
                    f.create_action_row(|f| {
                        f.create_button(|b| {
                            b.custom_id("remove")
                                .label("Remove")
                                .style(ButtonStyle::Secondary)
                        })
                        .create_button(|b| {
                            b.custom_id("switch")
                                .label("Switch")
                                .style(ButtonStyle::Secondary)
                        })
                    })
                })
            })
            .await
        {
            println!("Error sending message: {:?}", why);
        };

        if !msg.is_private() {
            // Delete message
            if let Err(why) = msg.delete(&context.http).await {
                println!("Error deleting message: {:?}", why);
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        // Check whether button has been pressed
        if interaction.kind() != InteractionType::MessageComponent {
            return;
        }

        let component = interaction.as_message_component().unwrap().clone();
        let custom_id = component.data.custom_id.to_string();

        if custom_id != "remove" && custom_id != "switch" {
            return;
        }

        // Make the Discord API happy no matter what :)
        component
        .create_interaction_response(&ctx.http, |r| {
            r.kind(InteractionResponseType::DeferredUpdateMessage)
        })
        .await
        .unwrap();

        let msg = &component.message;
        if !msg.author.bot {
            return;
        }

        let user = component.user.id.to_string();
        // Check whether user is correct
        if !msg.content.contains(&user) {
            return;
        }

        if custom_id == "remove" {
            if let Err(why) = component.edit_original_interaction_response(&ctx.http, |m| {
                m.content("💣 Deleted Message").allowed_mentions(|am| am.empty_parse());
                m.components(|c| c)
            })
            .await
            {
                println!("Error editing message: {:?}", why);
            }

            // Sleep for 5 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            // Delete the response message
            if let Err(why) = component.delete_original_interaction_response(&ctx.http)
            .await {
                println!("Error deleting message: {:?}", why);
            }
        } else {
            let mut new_msg = msg.content.clone();

            if UrlType::from_string(&new_msg) == UrlType::Fxtwitter {
                new_msg = convert_url(new_msg, UrlType::Fxtwitter, UrlType::Vxtwitter).await;
            } else {
                new_msg = convert_url(new_msg, UrlType::Vxtwitter, UrlType::Fxtwitter).await;
            }

            if let Err(why) = component.edit_original_interaction_response(&ctx.http, |m| {
                m.content(new_msg).allowed_mentions(|am| am.empty_parse())
            })
            .await {
                println!("Error editing message: {:?}", why);
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    // Load the environment variables from the .env file.
    dotenv().ok();

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
