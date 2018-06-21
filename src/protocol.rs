extern crate serde_json;
extern crate snap;

use actions;
use config::Config;
use std::{error::Error, io::prelude::*, path::Path};
use web_view::*;

pub struct Protocol;

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize)]
pub enum ProtocolKind {
    interop,
    ws,
}

pub struct View<'a, 'b: 'a> {
    pub webview: Option<&'a mut WebView<'b, ()>>,
}

impl<'a, 'b> View<'a, 'b> {
    pub fn new() -> View<'a, 'b> { View { webview: None::<&mut WebView<'b, ()>>, } }
    pub fn with_webview(&mut self, webview: &'a mut WebView<'b, ()>) -> &mut View<'a, 'b> {
        self.webview = Some(webview);
        self
    }
}

impl Protocol {
    pub fn new<T: Into<Config>>(config: T) -> Protocol {
        match config.into().protocol() {
            ProtocolKind::ws => Protocol::init_ws(),
            _ => Protocol::init_interop(),
        };
        Protocol {}
    }

    fn init_ws() {
        use ws::{listen, Message};
        listen("127.0.0.1:36767", |out| {
            move |msg: Message| {
                msg.as_text().map(|s| Protocol::handle(s, &mut View::new(), |m, _| out.send(m).map_err(Box::from)))
            }
        }).expect("Failed to create WebSocket")
    }

    fn init_interop() {
        let mut f = snap::Reader::new(Protocol::HTML);
        let mut buffer = String::new();
        f.read_to_string(&mut buffer).expect("can't inizialize view");
        let name = "telegramR";
        run(name,
            Content::Html(buffer),
            Some((900, 800)),
            true,
            false,
            |_| {},
            |webview, arg, _| {
                Protocol::handle(arg, &mut View::new().with_webview(webview), |m, v| {
                    Protocol::eval(m, v).map_err(Box::from)
                })
            },
            ());
    }

    fn handle<S>(msg: &str, view: &mut View, send: S)
        where S: FnOnce(String, &mut View) -> Result<(), Box<Error>> {
        if let Err(err) = actions::process(msg, view).map(|res| send(res, view)) {
            println!("error: {:?}", err);
        }
    }

    fn eval(s: String, view: &mut View) -> Result<(), &'static str> {
        view.webview.as_mut()
            .map(|v| {
                     v.eval(&format!("window.render({})", s));
                 })
            .ok_or("eval error")
    }

    const HTML: &'static [u8] = include_bytes!("d.sz");
}

impl<P: AsRef<Path>> From<P> for Config {
    fn from(path: P) -> Self { Config::read(path).unwrap_or_default() }
}
