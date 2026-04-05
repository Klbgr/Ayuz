use std::path::PathBuf;

use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use super::helpers::{icm_profil_anwenden, icm_profil_reset, setup_icm_profiles};
use crate::services::config::AppConfig;

fn dateiname_fuer_index(index: u32) -> Option<&'static str> {
    match index {
        1 => Some("ASUS_sRGB.icm"),
        2 => Some("ASUS_DCIP3.icm"),
        3 => Some("ASUS_DisplayP3.icm"),
        _ => None,
    }
}

// ── Farbskala (ComboRow) ─────────────────────────────────────────────────────

pub struct FarbskalaModel {
    farbskala_index: u32,
    icm_basis_pfad: Option<PathBuf>,
}

impl FarbskalaModel {
    fn farbskala_beschreibung(&self) -> std::borrow::Cow<'static, str> {
        match self.farbskala_index {
            1 => t!("farbskala_desc_srgb"),
            2 => t!("farbskala_desc_dcip3"),
            3 => t!("farbskala_desc_displayp3"),
            _ => t!("farbskala_desc_native"),
        }
    }
}

#[derive(Debug)]
pub enum FarbskalaMsg {
    FarbskalaWechseln(u32),
}

#[derive(Debug)]
pub enum FarbskalaCommandOutput {
    IcmBereit(PathBuf),
    ProfilAngewendet(u32),
    Fehler(String),
}

#[relm4::component(pub)]
impl Component for FarbskalaModel {
    type Init = ();
    type Input = FarbskalaMsg;
    type Output = String;
    type CommandOutput = FarbskalaCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("farbskala_group_title"),
            set_description: Some(&t!("farbskala_group_desc")),

            add = &adw::ComboRow {
                set_title: &t!("farbskala_title"),
                #[watch]
                set_subtitle: &model.farbskala_beschreibung(),
                set_model: Some(&farbskala_list),
                #[watch]
                set_selected: model.farbskala_index,
                connect_selected_notify[sender] => move |row| {
                    sender.input(FarbskalaMsg::FarbskalaWechseln(row.selected()));
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let config = AppConfig::load();

        let native = t!("farbskala_option_native");
        let farbskala_list = gtk::StringList::new(&[&native, "sRGB", "DCI-P3", "Display P3"]);

        let model = FarbskalaModel {
            farbskala_index: config.farbskala_index,
            icm_basis_pfad: None,
        };

        let widgets = view_output!();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    match setup_icm_profiles().await {
                        Ok(pfad) => out.emit(FarbskalaCommandOutput::IcmBereit(pfad)),
                        Err(e) => out.emit(FarbskalaCommandOutput::Fehler(e)),
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: FarbskalaMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            FarbskalaMsg::FarbskalaWechseln(index) => {
                if index == self.farbskala_index {
                    return;
                }
                self.farbskala_index = index;
                AppConfig::update(|c| c.farbskala_index = index);

                if let Some(basis) = self.icm_basis_pfad.clone() {
                    profil_anwenden(index, basis, &sender);
                } else {
                    eprintln!("{}", t!("farbskala_icm_path_not_ready"));
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: FarbskalaCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            FarbskalaCommandOutput::IcmBereit(pfad) => {
                eprintln!(
                    "{}",
                    t!("farbskala_icm_ready", path = pfad.display().to_string())
                );
                if self.farbskala_index > 0 {
                    profil_anwenden(self.farbskala_index, pfad.clone(), &sender);
                }
                self.icm_basis_pfad = Some(pfad);
            }
            FarbskalaCommandOutput::ProfilAngewendet(index) => {
                eprintln!(
                    "{}",
                    t!("farbskala_profile_applied", index = index.to_string())
                );
            }
            FarbskalaCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

fn profil_anwenden(index: u32, basis: PathBuf, sender: &ComponentSender<FarbskalaModel>) {
    sender.command(move |out, shutdown| {
        shutdown
            .register(async move {
                let ergebnis = match dateiname_fuer_index(index) {
                    None => icm_profil_reset().await,
                    Some(dateiname) => icm_profil_anwenden(dateiname, &basis).await,
                };
                match ergebnis {
                    Ok(()) => out.emit(FarbskalaCommandOutput::ProfilAngewendet(index)),
                    Err(e) => out.emit(FarbskalaCommandOutput::Fehler(e)),
                }
            })
            .drop_on_shutdown()
    });
}
