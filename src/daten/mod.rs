//! Lesen und Auswerten von `/proc` sowie Einheitenformatierung – ohne GTK,
//! damit alles mit abgelegten Beispieldateien (`tests/daten/`) prüfbar ist.

pub mod cpu;
pub mod dateisysteme;
pub mod einheiten;
pub mod netz;
pub mod prozesse;
pub mod speicher;
