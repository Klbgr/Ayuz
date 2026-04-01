use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::commands::run_command_blocking;
use crate::services::config::AppConfig;

const PRESET_MUSIK: &str = include_str!("../../../assets/presets/Music.json");
const PRESET_FILM: &str = include_str!("../../../assets/presets/Movie.json");
const PRESET_VIDEO: &str = include_str!("../../../assets/presets/Video.json");
const PRESET_SPRACHE: &str = include_str!("../../../assets/presets/Voice.json");
const PRESET_ANGEPASST: &str = include_str!("../../../assets/presets/Perfect_EQ.json");

const PRESETS: &[(&str, &str)] = &[
    ("Movie", PRESET_FILM),
    ("Music", PRESET_MUSIK),
    ("Perfect_EQ", PRESET_ANGEPASST),
    ("Video", PRESET_VIDEO),
    ("Voice", PRESET_SPRACHE),
];

// Index 0..5: Movie, Music, None(bypass), Perfect_EQ, Video, Voice
// Index 2 = None (kein Preset, nur Bypass an)
const NONE_IDX: u32 = 2;
const PRESET_NAMEN: &[&str] = &["Movie", "Music", "Perfect_EQ", "Video", "Voice"];

pub struct SoundModesModel {
    ee_installiert: bool,
    aktuelles_profil: u32,
    dropdown: gtk::DropDown,
    beschreibung: String,
}

#[derive(Debug)]
pub enum AudioMsg {
    ProfilWechseln(u32),
}

#[derive(Debug)]
pub enum AudioCommandOutput {
    EeGeprueft(bool),
    PresetsInstalliert,
    ProfilGesetzt(u32),
    Fehler(String),
}

#[relm4::component(pub)]
impl Component for SoundModesModel {
    type Init = ();
    type Input = AudioMsg;
    type Output = String;
    type CommandOutput = AudioCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("audio_profiles_title"),
            #[watch]
            set_description: Some(model.beschreibung.as_str()),

            add = &adw::ActionRow {
                set_title: &t!("audio_profile_label"),
                add_suffix = &model.dropdown.clone(),
                set_activatable_widget: Some(&model.dropdown),
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let config = AppConfig::load();

        let optionen = gtk::StringList::new(&[
            &t!("audio_profile_film"),
            &t!("audio_profile_musik"),
            &t!("audio_profile_none"),
            &t!("audio_profile_optimiert"),
            &t!("audio_profile_video"),
            &t!("audio_profile_sprache"),
        ]);
        let dropdown = gtk::DropDown::new(Some(optionen), gtk::Expression::NONE);
        dropdown.set_valign(gtk::Align::Center);
        dropdown.set_selected(config.audio_profil);
        dropdown.set_sensitive(false); // bis EE-Check abgeschlossen

        {
            let sender = sender.clone();
            dropdown.connect_selected_notify(move |dd| {
                sender.input(AudioMsg::ProfilWechseln(dd.selected()));
            });
        }

        let model = SoundModesModel {
            ee_installiert: false,
            aktuelles_profil: config.audio_profil,
            dropdown,
            beschreibung: t!("audio_profiles_desc").to_string(),
        };

        let widgets = view_output!();

        // EasyEffects-Check
        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    let installiert = tokio::task::spawn_blocking(|| {
                        std::process::Command::new("which")
                            .arg("easyeffects")
                            .status()
                            .map(|s| s.success())
                            .unwrap_or(false)
                    })
                    .await
                    .unwrap_or(false);
                    out.emit(AudioCommandOutput::EeGeprueft(installiert));
                })
                .drop_on_shutdown()
        });

        // Presets installieren
        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    match presets_installieren().await {
                        Ok(()) => out.emit(AudioCommandOutput::PresetsInstalliert),
                        Err(e) => out.emit(AudioCommandOutput::Fehler(
                            t!("audio_preset_install_error", error = e).to_string(),
                        )),
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: AudioMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            AudioMsg::ProfilWechseln(idx) => {
                if idx == self.aktuelles_profil {
                    return;
                }
                self.aktuelles_profil = idx;
                AppConfig::update(|c| c.audio_profil = idx);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            if let Err(e) = easyeffects_profil_setzen(idx).await {
                                out.emit(AudioCommandOutput::Fehler(e));
                                return;
                            }
                            out.emit(AudioCommandOutput::ProfilGesetzt(idx));
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: AudioCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AudioCommandOutput::EeGeprueft(installiert) => {
                self.ee_installiert = installiert;
                self.dropdown.set_sensitive(installiert);
                self.beschreibung = if installiert {
                    t!("audio_profiles_desc").to_string()
                } else {
                    t!("ee_missing_warning").to_string()
                };
            }
            AudioCommandOutput::PresetsInstalliert => {
                eprintln!("Audio-Presets installiert.");
            }
            AudioCommandOutput::ProfilGesetzt(idx) => {
                eprintln!("{}", t!("audio_profile_set", profile = idx));
            }
            AudioCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

async fn easyeffects_profil_setzen(idx: u32) -> Result<(), String> {
    // Prüfen ob der Daemon bereits läuft
    let daemon_laeuft = tokio::task::spawn_blocking(|| {
        std::process::Command::new("pgrep")
            .args(["-x", "easyeffects"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false);

    if !daemon_laeuft {
        // Daemon non-blocking starten (kein GUI, nur Service)
        let _ = tokio::process::Command::new("easyeffects")
            .arg("--gapplication-service")
            .spawn();
        // Warten bis der Daemon bereit ist
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    }

    if idx == NONE_IDX {
        run_command_blocking("easyeffects", &["-b", "1"]).await?;
    } else {
        run_command_blocking("easyeffects", &["-b", "2"]).await?;
        let preset_idx = if idx < NONE_IDX { idx } else { idx - 1 } as usize;
        run_command_blocking("easyeffects", &["-l", PRESET_NAMEN[preset_idx]]).await?;
    }

    Ok(())
}

async fn presets_installieren() -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let dir = std::path::PathBuf::from(&home).join(".config/easyeffects/output");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;
    for (name, content) in PRESETS {
        let path = dir.join(format!("{}.json", name));
        if !path.exists() {
            tokio::fs::write(&path, content)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
