use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::commands::run_command_blocking;
use crate::services::config::AppConfig;

pub struct FnKeyModel {
    gesperrt: bool,
    zeile_hinweis: adw::ActionRow,
    zeile_gesperrt: adw::ActionRow,
    zeile_normal: adw::ActionRow,
}

#[derive(Debug)]
pub enum FnKeyMsg {
    GesperrtUmschalten(bool),
}

#[derive(Debug)]
pub enum FnKeyCommandOutput {
    Gesetzt(bool),
    Fehler(String),
}

#[relm4::component(pub)]
impl Component for FnKeyModel {
    type Init = ();
    type Input = FnKeyMsg;
    type Output = String;
    type CommandOutput = FnKeyCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("fn_key_group_title"),
            set_description: Some(&t!("fn_key_group_desc")),

            add = &model.zeile_hinweis.clone(),
            add = &model.zeile_gesperrt.clone(),
            add = &model.zeile_normal.clone(),
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let check_gesperrt = gtk::CheckButton::new();
        let check_normal = gtk::CheckButton::new();

        check_normal.set_group(Some(&check_gesperrt));

        let gesperrt = AppConfig::load().input_fn_key_gesperrt;
        if gesperrt {
            check_gesperrt.set_active(true);
        } else {
            check_normal.set_active(true);
        }

        {
            let sender = sender.clone();
            check_gesperrt.connect_toggled(move |b| {
                if b.is_active() {
                    sender.input(FnKeyMsg::GesperrtUmschalten(true));
                }
            });
        }
        {
            let sender = sender.clone();
            check_normal.connect_toggled(move |b| {
                if b.is_active() {
                    sender.input(FnKeyMsg::GesperrtUmschalten(false));
                }
            });
        }

        let zeile_hinweis = adw::ActionRow::new();
        zeile_hinweis.set_title(&t!("fn_key_hint_title"));
        zeile_hinweis.set_subtitle(&t!("fn_key_hint_subtitle"));
        zeile_hinweis.set_selectable(false);

        let zeile_gesperrt = adw::ActionRow::new();
        zeile_gesperrt.set_title(&t!("fn_key_locked_title"));
        zeile_gesperrt.set_subtitle(&t!("fn_key_locked_subtitle"));
        zeile_gesperrt.add_prefix(&check_gesperrt);
        zeile_gesperrt.set_activatable_widget(Some(&check_gesperrt));

        let zeile_normal = adw::ActionRow::new();
        zeile_normal.set_title(&t!("fn_key_normal_title"));
        zeile_normal.set_subtitle(&t!("fn_key_normal_subtitle"));
        zeile_normal.add_prefix(&check_normal);
        zeile_normal.set_activatable_widget(Some(&check_normal));

        let model = FnKeyModel {
            gesperrt,
            zeile_hinweis,
            zeile_gesperrt,
            zeile_normal,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: FnKeyMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            FnKeyMsg::GesperrtUmschalten(gesperrt) => {
                if gesperrt == self.gesperrt {
                    return;
                }
                self.gesperrt = gesperrt;

                let args_flag = format!(
                    "--args=asus_wmi.fnlock_default={}",
                    if gesperrt { "0" } else { "1" }
                );

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            let result = run_command_blocking(
                                "pkexec",
                                &[
                                    "grubby",
                                    "--update-kernel=ALL",
                                    "--remove-args=asus_wmi.fnlock_default",
                                    &args_flag,
                                ],
                            )
                            .await;

                            match result {
                                Ok(()) => out.emit(FnKeyCommandOutput::Gesetzt(gesperrt)),
                                Err(e) => out.emit(FnKeyCommandOutput::Fehler(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: FnKeyCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            FnKeyCommandOutput::Gesetzt(gesperrt) => {
                AppConfig::update(|c| c.input_fn_key_gesperrt = gesperrt);
                let mode = if gesperrt {
                    t!("fn_key_mode_locked")
                } else {
                    t!("fn_key_mode_normal")
                };
                self.zeile_hinweis
                    .set_subtitle(&t!("fn_key_saved", mode = mode));
            }
            FnKeyCommandOutput::Fehler(e) => {
                self.zeile_hinweis
                    .set_subtitle(&t!("fn_key_save_error", error = e.clone()));
                let _ = sender.output(e);
            }
        }
    }
}
