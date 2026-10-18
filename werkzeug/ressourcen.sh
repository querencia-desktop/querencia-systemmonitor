#!/bin/sh
# Erzeugt data/resource/systemmonitor.gresource aus systemmonitor.gresource.xml
# (Bauzeit-Artefakt, eingecheckt, damit der Bau ohne glib-compile-resources geht).
cd "$(dirname "$0")/.." && glib-compile-resources --sourcedir=data/resource \
    --target=data/resource/systemmonitor.gresource data/resource/systemmonitor.gresource.xml
