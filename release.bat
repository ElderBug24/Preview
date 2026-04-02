@echo off

cargo build --release
copy .\target\release\preview.exe C:\Users\R3D\aliases\preview.exe

