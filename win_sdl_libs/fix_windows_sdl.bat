REM This script aims to fix SDL linking for the MinGW toolchain on Windows

REM How to use: extract 'https://www.libsdl.org/release/SDL2-devel-2.0.9-mingw.tar.gz' and move all files in its folders
REM 	'SDL2-2.0.9\x86_64-w64-mingw32\bin' and 'SDL2-2.0.9\x86_64-w64-mingw32\lib' into here
REM 	namely: libSDL2.a libSDL2.dll.a libSDL2.la libSDL2_test.a libSDL2_test.la libSDL2main.a libSDL2main.la SDL2.dll

FOR /F "tokens=1 delims=" %%A in ('rustup default') do SET result=%%A
cp *.la C:\Users\%USERNAME%\.rustup\toolchains\nightly-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\lib\
cp *.a C:\Users\%USERNAME%\.rustup\toolchains\nightly-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\lib\

REM for easy execution, assuming the working directoy is in there
cp SDL2.dll ../target/debug/SDL2.dll
cp SDL2.dll ../target/release/SDL2.dll
pause