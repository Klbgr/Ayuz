use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use super::helpers::{kwriteconfig_ausfuehren, qdbus_ausfuehren};
use crate::services::config::AppConfig;

pub struct ZielmodusModel {
    aktiv: bool,
    kde_verfuegbar: bool,
}

#[derive(Debug)]
pub enum ZielmodusMsg {
    AktivSetzen(bool),
}

#[derive(Debug)]
pub enum ZielmodusCommandOutput {
    AktivGesetzt(bool),
    Fehler(String),
}

#[relm4::component(pub)]
impl Component for ZielmodusModel {
    type Init = ();
    type Input = ZielmodusMsg;
    type Output = String;
    type CommandOutput = ZielmodusCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("zielmodus_group_title"),
            set_description: Some(&t!("zielmodus_group_desc")),

            add = &gtk::Label {
                #[watch]
                set_visible: !model.kde_verfuegbar,
                set_label: &t!("zielmodus_kde_required"),
                add_css_class: "error",
                set_wrap: true,
                set_xalign: 0.0,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_bottom: 4,
            },

            add = &adw::SwitchRow {
                set_title: &t!("zielmodus_switch_title"),
                set_subtitle: &t!("zielmodus_switch_subtitle"),

                #[watch]
                set_active: model.aktiv,
                #[watch]
                set_sensitive: model.kde_verfuegbar,

                connect_active_notify[sender] => move |switch| {
                    sender.input(ZielmodusMsg::AktivSetzen(switch.is_active()));
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut config = AppConfig::load();
        let kde_verfuegbar = ist_kde();

        let aktiv = if kde_verfuegbar {
            let a =
                lese_kwin_bool("Plugins", "diminactiveEnabled").unwrap_or(config.zielmodus_aktiv);
            config.zielmodus_aktiv = a;
            config.save();
            a
        } else {
            config.zielmodus_aktiv
        };

        let model = ZielmodusModel {
            aktiv,
            kde_verfuegbar,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: ZielmodusMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            ZielmodusMsg::AktivSetzen(aktiv) => {
                if aktiv == self.aktiv {
                    return;
                }
                self.aktiv = aktiv;
                AppConfig::update(|c| c.zielmodus_aktiv = aktiv);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match kwin_effekt_setzen(aktiv).await {
                                Ok(()) => out.emit(ZielmodusCommandOutput::AktivGesetzt(aktiv)),
                                Err(e) => out.emit(ZielmodusCommandOutput::Fehler(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: ZielmodusCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ZielmodusCommandOutput::AktivGesetzt(aktiv) => {
                eprintln!("{}", t!("zielmodus_aktiv_set", value = aktiv.to_string()));
            }
            ZielmodusCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

async fn kwin_effekt_setzen(aktiv: bool) -> Result<(), String> {
    let wert = if aktiv { "true" } else { "false" };
    kwriteconfig_ausfuehren(&[
        "--file",
        "kwinrc",
        "--group",
        "Plugins",
        "--key",
        "diminactiveEnabled",
        "--type",
        "bool",
        wert,
    ])
    .await?;

    let method = if aktiv { "loadEffect" } else { "unloadEffect" };
    qdbus_ausfuehren(vec![
        "org.kde.KWin".to_string(),
        "/Effects".to_string(),
        method.to_string(),
        "diminactive".to_string(),
    ])
    .await
}

fn ist_kde() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_uppercase().contains("KDE"))
        .unwrap_or(false)
}

fn lese_kwin_bool(group: &str, key: &str) -> Option<bool> {
    let output = std::process::Command::new("kreadconfig6")
        .args([
            "--file",
            "kwinrc",
            "--group",
            group,
            "--key",
            key,
            "--default",
            "false",
        ])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_lowercase();
    Some(s == "true")
}
