let SessionLoad = 1
let s:so_save = &g:so | let s:siso_save = &g:siso | setg so=0 siso=0 | setl so=-1 siso=-1
let v:this_session=expand("<sfile>:p")
silent only
silent tabonly
cd ~/work/rustbee
if expand('%') == '' && !&modified && line('$') <= 1 && getline(1) == ''
  let s:wipebuf = bufnr('%')
endif
let s:shortmess_save = &shortmess
if &shortmess =~ 'A'
  set shortmess=aoOA
else
  set shortmess=aoO
endif
badd +21 README.md
badd +20 .gitignore
badd +9 Cargo.toml
badd +27 rustbee-common/src/lib.rs
badd +17 rustbee-common/src/constants.rs
badd +65 src/main.rs
badd +333 rustbee-daemon/src/main.rs
badd +75 rustbee-common/src/colors.rs
badd +329 src/cli.rs
badd +11 rustbee-common/Cargo.toml
badd +32 rustbee-common/src/tests.rs
badd +38 Justfile
badd +12 rustbee-daemon/Justfile
badd +10 rustbee-daemon/Cargo.toml
badd +37 rustbee-common/librustbee.h
badd +166 rustbee-common/src/ffi.rs
badd +15 rustbee-common/src/storage.rs
badd +9 src/address.rs
badd +52 rustbee-common/src/logger.rs
badd +53 .github/workflows/build_and_release.yml
badd +12 CHANGELOG.md
badd +22 TODO.md
badd +74 rustbee-common/src/windows/bluetooth.rs
badd +184 rustbee-common/src/device.rs
badd +1 rustbee-common/src/linux/mod.rs
badd +173 rustbee-common/src/linux/device.rs
badd +60 rustbee-common/src/windows/device.rs
badd +2 rustbee-common/src/windows/mod.rs
badd +16 rustbee-common/src/utils/mod.rs
badd +94 rustbee-common/src/windows/daemon.rs
badd +1 dist/install_win.bat
badd +13 .github/dependabot.yml
badd +6 .github/workflows/clippy_lint.yml
badd +1 rustbee-gui/package.json
badd +3 .ignore
badd +28 rustbee-gui/src/routes/+page.svelte
badd +26 rustbee-gui/src-tauri/src/main.rs
badd +16 rustbee-gui/src-tauri/tauri.conf.json
badd +1364 ~/work/old_rustbee-gui/rustbee-gui/src/main.rs
badd +3 rustbee-gui/src-tauri/Cargo.toml
badd +56 rustbee-gui/src-tauri/src/state.rs
badd +74 rustbee-gui/src-tauri/src/commands.rs
badd +1 rustbee-gui/Justfile
badd +50 rustbee-gui/src/lib/types.ts
badd +157 rustbee-gui/src/components/header.svelte
badd +10 rustbee-gui/tailwind.config.js
badd +10 rustbee-gui/src/styles/tailwind.css
badd +4 rustbee-gui/postcss.config.js
badd +47 rustbee-gui/src/components/subheader.svelte
badd +26 rustbee-gui/src/components/device.svelte
badd +40 rustbee-gui/src/lib/stores/caller.ts
badd +1 rustbee-gui/.prettierignore
badd +29 rustbee-gui/src/lib/utils.ts
badd +9 rustbee-gui/src/app.html
badd +4 rustbee-gui/src/routes/+layout.svelte
argglobal
%argdel
edit rustbee-common/src/device.rs
wincmd t
let s:save_winminheight = &winminheight
let s:save_winminwidth = &winminwidth
set winminheight=0
set winheight=1
set winminwidth=0
set winwidth=1
argglobal
balt TODO.md
setlocal fdm=manual
setlocal fde=0
setlocal fmr={{{,}}}
setlocal fdi=#
setlocal fdl=0
setlocal fml=1
setlocal fdn=20
setlocal fen
silent! normal! zE
let &fdl = &fdl
let s:l = 184 - ((31 * winheight(0) + 31) / 63)
if s:l < 1 | let s:l = 1 | endif
keepjumps exe s:l
normal! zt
keepjumps 184
normal! 0
lcd ~/work/rustbee
tabnext 1
if exists('s:wipebuf') && len(win_findbuf(s:wipebuf)) == 0 && getbufvar(s:wipebuf, '&buftype') isnot# 'terminal'
  silent exe 'bwipe ' . s:wipebuf
endif
unlet! s:wipebuf
set winheight=1 winwidth=20
let &shortmess = s:shortmess_save
let &winminheight = s:save_winminheight
let &winminwidth = s:save_winminwidth
let s:sx = expand("<sfile>:p:r")."x.vim"
if filereadable(s:sx)
  exe "source " . fnameescape(s:sx)
endif
let &g:so = s:so_save | let &g:siso = s:siso_save
doautoall SessionLoadPost
unlet SessionLoad
" vim: set ft=vim :
