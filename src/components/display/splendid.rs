use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;

use super::helpers::qdbus_ausfuehren;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplendidProfil {
    Normal,
    Lebendig,
    Manuell,
    EyeCare,
    EReading,
}

#[allow(dead_code)]
pub struct SplendidModel {
    aktuelles_profil: SplendidProfil,
    farbtemperatur: f64,
    eye_care_staerke: f64,
    check_normal: gtk::CheckButton,
    check_lebendig: gtk::CheckButton,
    check_manuell: gtk::CheckButton,
    check_eye_care: gtk::CheckButton,
    check_e_reading: gtk::CheckButton,
    scale_farbtemperatur: gtk::Scale,
    scale_eye_care: gtk::Scale,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum SplendidMsg {
    ProfilWechseln(SplendidProfil),
    FarbtemperaturGeaendert(f64),
    EyeCareStaerkeGeaendert(f64),
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum SplendidCommandOutput {
    EyeCareGesetzt(bool),
    FarbtemperaturGesetzt(u32),
    Fehler(String),
}

#[relm4::component(pub)]
impl Component for SplendidModel {
    type Init = ();
    type Input = SplendidMsg;
    type Output = ();
    type CommandOutput = SplendidCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: "Splendid",

            add = &adw::ActionRow {
                set_title: "Normal",
                add_prefix = &model.check_normal.clone(),
                set_activatable_widget: Some(&model.check_normal),
            },

            add = &adw::ActionRow {
                set_title: "Lebendig",
                add_prefix = &model.check_lebendig.clone(),
                set_activatable_widget: Some(&model.check_lebendig),
            },

            add = &adw::ActionRow {
                set_title: "Manuell",
                add_prefix = &model.check_manuell.clone(),
                set_activatable_widget: Some(&model.check_manuell),
            },

            add = &adw::ActionRow {
                set_title: "Farbtemperatur",
                add_suffix = &model.scale_farbtemperatur.clone(),

                #[watch]
                set_visible: model.aktuelles_profil == SplendidProfil::Manuell,
            },

            add = &adw::ActionRow {
                set_title: "Eye Care",
                add_prefix = &model.check_eye_care.clone(),
                set_activatable_widget: Some(&model.check_eye_care),
            },

            add = &adw::ActionRow {
                set_title: "Stärke",
                add_suffix = &model.scale_eye_care.clone(),

                #[watch]
                set_visible: model.aktuelles_profil == SplendidProfil::EyeCare,
            },

            add = &adw::ActionRow {
                set_title: "E-Reading",
                set_subtitle: "Graustufen",
                add_prefix = &model.check_e_reading.clone(),
                set_activatable_widget: Some(&model.check_e_reading),
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let check_normal = gtk::CheckButton::new();
        let check_lebendig = gtk::CheckButton::new();
        let check_manuell = gtk::CheckButton::new();
        let check_eye_care = gtk::CheckButton::new();
        let check_e_reading = gtk::CheckButton::new();

        check_lebendig.set_group(Some(&check_normal));
        check_manuell.set_group(Some(&check_normal));
        check_eye_care.set_group(Some(&check_normal));
        check_e_reading.set_group(Some(&check_normal));
        check_normal.set_active(true);

        for (btn, profil) in [
            (&check_normal, SplendidProfil::Normal),
            (&check_lebendig, SplendidProfil::Lebendig),
            (&check_manuell, SplendidProfil::Manuell),
            (&check_eye_care, SplendidProfil::EyeCare),
            (&check_e_reading, SplendidProfil::EReading),
        ] {
            let sender = sender.clone();
            btn.connect_toggled(move |b| {
                if b.is_active() {
                    sender.input(SplendidMsg::ProfilWechseln(profil));
                }
            });
        }

        let scale_farbtemperatur =
            gtk::Scale::with_range(gtk::Orientation::Horizontal, 2000.0, 6500.0, 100.0);
        scale_farbtemperatur.set_hexpand(true);
        scale_farbtemperatur.set_width_request(300);
        scale_farbtemperatur.set_valign(gtk::Align::Center);
        scale_farbtemperatur.set_value(4500.0);

        let scale_eye_care = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
        scale_eye_care.set_hexpand(true);
        scale_eye_care.set_width_request(300);
        scale_eye_care.set_valign(gtk::Align::Center);

        {
            let sender = sender.clone();
            scale_farbtemperatur.connect_value_changed(move |s| {
                sender.input(SplendidMsg::FarbtemperaturGeaendert(s.value()));
            });
        }
        {
            let sender = sender.clone();
            scale_eye_care.connect_value_changed(move |s| {
                sender.input(SplendidMsg::EyeCareStaerkeGeaendert(s.value()));
            });
        }

        let model = SplendidModel {
            aktuelles_profil: SplendidProfil::Normal,
            farbtemperatur: 4500.0,
            eye_care_staerke: 0.0,
            check_normal,
            check_lebendig,
            check_manuell,
            check_eye_care,
            check_e_reading,
            scale_farbtemperatur,
            scale_eye_care,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: SplendidMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            SplendidMsg::ProfilWechseln(profil) => {
                if profil == self.aktuelles_profil {
                    return;
                }
                let vorheriges = self.aktuelles_profil;
                self.aktuelles_profil = profil;

                // Night Color deaktivieren, wenn wir Eye Care verlassen
                if vorheriges == SplendidProfil::EyeCare && profil != SplendidProfil::EyeCare {
                    sender.command(|out, shutdown| {
                        shutdown
                            .register(async move {
                                night_color_setzen(false, &out).await;
                            })
                            .drop_on_shutdown()
                    });
                }

                match profil {
                    SplendidProfil::EyeCare => {
                        sender.command(|out, shutdown| {
                            shutdown
                                .register(async move {
                                    night_color_setzen(true, &out).await;
                                })
                                .drop_on_shutdown()
                        });
                    }
                    SplendidProfil::Normal => {
                        eprintln!("Splendid: Normal-Profil aktiviert (Standard-Farbwiedergabe)");
                    }
                    SplendidProfil::Lebendig => {
                        eprintln!(
                            "Splendid: Lebendig-Profil aktiviert – ICC-Profil muss in KDE-Einstellungen hinterlegt werden"
                        );
                    }
                    SplendidProfil::Manuell => {
                        eprintln!(
                            "Splendid: Manuell-Profil aktiviert – Farbtemperatur über Slider einstellen"
                        );
                    }
                    SplendidProfil::EReading => {
                        eprintln!(
                            "Splendid: E-Reading (Graustufen) aktiviert – ICC-Profil muss in KDE-Einstellungen hinterlegt werden"
                        );
                    }
                }
            }
            SplendidMsg::FarbtemperaturGeaendert(wert) => {
                self.farbtemperatur = wert;
                let kelvin = wert as u32;

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            farbtemperatur_setzen(kelvin, &out).await;
                        })
                        .drop_on_shutdown()
                });
            }
            SplendidMsg::EyeCareStaerkeGeaendert(wert) => {
                self.eye_care_staerke = wert;
                eprintln!("Eye Care Stärke auf {} gesetzt", wert);
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: SplendidCommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            SplendidCommandOutput::EyeCareGesetzt(aktiv) => {
                eprintln!(
                    "Eye Care Night Color auf {} gesetzt",
                    if aktiv { "aktiv" } else { "inaktiv" }
                );
            }
            SplendidCommandOutput::FarbtemperaturGesetzt(kelvin) => {
                eprintln!("Farbtemperatur auf {}K gesetzt", kelvin);
            }
            SplendidCommandOutput::Fehler(e) => {
                eprintln!("Fehler: {e}");
            }
        }
    }
}

#[allow(dead_code)]
/// Setzt die Farbtemperatur über kwriteconfig6 und toggelt Night Color zum Neuladen.
async fn farbtemperatur_setzen(kelvin: u32, out: &relm4::Sender<SplendidCommandOutput>) {
    let kelvin_str = kelvin.to_string();

    // Schritt 1: Farbtemperatur in kwinrc schreiben
    let args_kelvin = kelvin_str.clone();
    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new("kwriteconfig6")
            .args([
                "--file",
                "kwinrc",
                "--group",
                "NightColor",
                "--key",
                "NightTemperature",
                &args_kelvin,
            ])
            .status()
    })
    .await;

    match result {
        Ok(Ok(status)) if status.success() => {}
        Ok(Ok(status)) => {
            out.emit(SplendidCommandOutput::Fehler(format!(
                "kwriteconfig6 NightTemperature fehlgeschlagen mit Exit-Code: {}",
                status.code().unwrap_or(-1)
            )));
            return;
        }
        Ok(Err(e)) => {
            out.emit(SplendidCommandOutput::Fehler(format!(
                "kwriteconfig6 starten fehlgeschlagen: {e}"
            )));
            return;
        }
        Err(e) => {
            out.emit(SplendidCommandOutput::Fehler(format!(
                "spawn_blocking fehlgeschlagen: {e}"
            )));
            return;
        }
    }

    // Schritt 2: Night Color kurz deaktivieren und wieder aktivieren, um die neue Temperatur zu laden
    night_color_toggle(false, out).await;
    night_color_toggle(true, out).await;

    out.emit(SplendidCommandOutput::FarbtemperaturGesetzt(kelvin));
}

#[allow(dead_code)]
/// Setzt Night Color an/aus via qdbus (für Eye Care).
async fn night_color_setzen(aktiv: bool, out: &relm4::Sender<SplendidCommandOutput>) {
    let wert = if aktiv { "true" } else { "false" };
    let args = vec![
        "org.kde.KWin".to_string(),
        "/ColorCorrect".to_string(),
        "org.kde.kwin.ColorCorrect.nightColorEnabled".to_string(),
        wert.to_string(),
    ];
    match qdbus_ausfuehren(args).await {
        Ok(()) => out.emit(SplendidCommandOutput::EyeCareGesetzt(aktiv)),
        Err(e) => out.emit(SplendidCommandOutput::Fehler(e)),
    }
}

#[allow(dead_code)]
/// Interne Hilfsfunktion: Night Color toggle ohne Erfolgs-Emit (für Farbtemperatur-Reload).
async fn night_color_toggle(aktiv: bool, out: &relm4::Sender<SplendidCommandOutput>) {
    let wert = if aktiv { "true" } else { "false" };
    let args = vec![
        "org.kde.KWin".to_string(),
        "/ColorCorrect".to_string(),
        "org.kde.kwin.ColorCorrect.nightColorEnabled".to_string(),
        wert.to_string(),
    ];
    if let Err(e) = qdbus_ausfuehren(args).await {
        out.emit(SplendidCommandOutput::Fehler(e));
    }
}
