// Empêche l'ouverture d'une console sous Windows en release (sans effet sous Linux).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sxm_mail_lib::run()
}
