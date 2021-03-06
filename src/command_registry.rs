use failure::Error;
use std::collections::HashMap;
use super::{Context, Flow, Command};
use std::iter;
use std::sync::Arc;
use futures::future::LocalFutureObj;
use std::future::Future;
use crate::Message;

pub(crate) struct CommandRegistry {
    command_prefix: String,
    named_handlers: HashMap<String, Box<for<'a> Fn(&'a Context, &'a [&str]) -> LocalFutureObj<'a, Flow>>>,
    fallback_handlers: Vec<Box<for<'a> Fn(&'a Context) -> LocalFutureObj<'a, Flow>>>,
}

impl CommandRegistry {
    pub fn new(command_prefix: impl Into<String>) -> Self {
        Self {
            command_prefix: command_prefix.into(),
            named_handlers: HashMap::new(),
            fallback_handlers: Vec::new(),
        }
    }

    pub fn set_named_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl for<'a> Fn(&'a Context, &'a [&str]) -> LocalFutureObj<'a, Flow> + 'static,
    ) {
        self.named_handlers.insert(name.into(), Box::new(handler));  
    }

    pub fn add_fallback_handler(
        &mut self,
        handler: impl for<'a> Fn(&'a Context) -> LocalFutureObj<'a, Flow> + 'static,
    ) {
        self.fallback_handlers.push(Box::new(handler));
    }

    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    pub fn handle_message<'a>(self: Arc<Self>, message: &'a Message) -> impl Future<Output = Result<(), Error>> + 'a {
        async move {
            let context = match Context::new(message) {
                Some(context) => context,
                None => return Ok(()),
            };

            // Handle the main context first
            if let Some(command) = Command::parse(&self.command_prefix, context.body()) {
                if let Some(handler) = self.named_handlers.get(command.name()) {
                    if await!(handler(&context, command.args())) == Flow::Break {
                        return Ok(());
                    }
                }
            }

            // Then handle ALL inline contexts before deciding flow
            let contexts = iter::once(context.clone()).chain(context.inline_contexts());
            let mut any_inline_command_succeded = false;
            for context in contexts.take(3) {
                if let Some(command) = Command::parse(&self.command_prefix, context.body()) {
                    if let Some(handler) = self.named_handlers.get(command.name()) {
                        if await!(handler(&context, command.args())) == Flow::Break {
                            any_inline_command_succeded = true;
                        }
                    }
                }
            }

            if any_inline_command_succeded {
                return Ok(());
            }

            for handler in &self.fallback_handlers {
                if await!(handler(&context)) == Flow::Break {
                    return Ok(());
                }
            }

            Ok(())
        }
    }
}
