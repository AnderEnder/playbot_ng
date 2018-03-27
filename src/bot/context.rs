use irc;
use irc::client::prelude::*;

type SendFn = fn(&IrcClient, &str, &str) -> irc::error::Result<()>;

#[derive(Clone)]
pub struct Context<'a> {
    body: &'a str,
    is_directly_addressed: bool,
    is_ctcp: bool,
    send_fn: SendFn,
    source: &'a str,
    source_nickname: &'a str,
    target: &'a str,
    client: &'a IrcClient,
    current_nickname: &'a str,
}

impl<'a> Context<'a> {
    pub fn new(client: &'a IrcClient, message: &'a Message) -> Option<Self> {
        let mut body = match message.command {
            Command::PRIVMSG(_, ref body) => body.trim(),
            _ => return None,
        };

        let source_nickname = message.source_nickname()?;

        let is_ctcp = body.len() >= 2 && body.chars().next() == Some('\x01')
            && body.chars().last() == Some('\x01');

        if is_ctcp {
            body = &body[1..body.len() - 1];
        }

        let source = message.prefix.as_ref().map(<_>::as_ref)?;

        let target = match message.response_target() {
            Some(target) => target,
            None => {
                eprintln!("Unknown response target");
                return None;
            }
        };

        let is_directly_addressed = {
            let current_nickname = client.current_nickname();

            if body.starts_with(current_nickname) {
                let new_body = body[current_nickname.len()..].trim_left();

                if new_body.starts_with(":") || new_body.starts_with(",") {
                    body = new_body[1..].trim_left();
                    true
                } else {
                    false
                }
            } else {
                !target.is_channel_name()
            }
        };

        let send_fn: SendFn = match target.is_channel_name() {
            true => |client, target, message| client.send_notice(target, message),
            false => |client, target, message| client.send_privmsg(target, message),
        };

        let current_nickname = client.current_nickname();

        Some(Self {
            client,
            body,
            send_fn,
            source,
            source_nickname,
            target,
            is_directly_addressed,
            is_ctcp,
            current_nickname
        })
    }

    pub fn body(&self) -> &str {
        self.body
    }

    /// Wether the message was aimed directetly at the bot,
    /// either via private message or by prefixing a channel message with
    /// the bot's name, followed by ',' or ':'.
    pub fn is_directly_addressed(&self) -> bool {
        self.is_directly_addressed
    }

    pub fn is_ctcp(&self) -> bool {
        self.is_ctcp
    }

    pub fn reply<S: AsRef<str>>(&self, message: S) {
        (self.send_fn)(self.client, self.target, message.as_ref());
    }

    pub fn source(&self) -> &str {
        self.source
    }

    pub fn source_nickname(&self) -> &str {
        self.source_nickname
    }

    pub fn current_nickname(&self) -> &str {
        self.current_nickname
    }
}