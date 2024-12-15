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
badd +100 rustbee
badd +4 .gitignore
badd +9 Cargo.toml
badd +27 rustbee-common/src/lib.rs
badd +18 rustbee-common/src/constants.rs
badd +65 src/main.rs
badd +249 rustbee-daemon/src/main.rs
badd +233 rustbee-common/src/colors.rs
badd +153 src/cli.rs
badd +10 rustbee-common/Cargo.toml
badd +32 rustbee-common/src/tests.rs
badd +1 Justfile
badd +12 rustbee-daemon/Justfile
badd +50 rustbee-common/src/daemon.rs
badd +10 rustbee-daemon/Cargo.toml
badd +8 rustbee-common/librustbee.h
badd +56 rustbee-common/src/ffi.rs
badd +15 rustbee-common/src/storage.rs
badd +9 src/address.rs
badd +52 rustbee-common/src/logger.rs
badd +53 .github/workflows/build_and_release.yml
badd +11 CHANGELOG.md
badd +1 TODO.md
badd +74 rustbee-common/src/windows/bluetooth.rs
badd +33 rustbee-common/src/device.rs
badd +1 rustbee-common/src/linux/mod.rs
badd +120 rustbee-common/src/linux/device.rs
badd +60 rustbee-common/src/windows/device.rs
badd +2 rustbee-common/src/windows/mod.rs
badd +16 rustbee-common/src/utils/mod.rs
badd +94 rustbee-common/src/windows/daemon.rs
badd +1 dist/install_win.bat
badd +13 .github/dependabot.yml
badd +6 .github/workflows/clippy_lint.yml
badd +1 rustbee-gui/package.json
badd +2 .ignore
badd +42 rustbee-gui/src/routes/+page.svelte
badd +89 rustbee-gui/src-tauri/src/main.rs
badd +16 rustbee-gui/src-tauri/tauri.conf.json
badd +39 ~/work/old_rustbee-gui/rustbee-gui/src/main.rs
badd +3 rustbee-gui/src-tauri/Cargo.toml
badd +68 rustbee-gui/src-tauri/src/state.rs
badd +42 rustbee-gui/src-tauri/src/commands.rs
badd +17 rustbee-gui/Justfile
badd +15 rustbee-gui/src/lib/types.ts
badd +80 rustbee-gui/node_modules/.pnpm/svelte-awesome-color-picker@3.1.4_svelte@5.2.7/node_modules/svelte-awesome-color-picker/dist/components/ColorPicker.svelte.d.ts
badd +9 rustbee-gui/src/components/header.svelte
badd +8 rustbee-gui/tailwind.config.js
badd +3 rustbee-gui/src/styles/tailwind.css
badd +5 rustbee-gui/src/routes/+layout.svelte
argglobal
%argdel
edit rustbee-gui/src/routes/+page.svelte
wincmd t
let s:save_winminheight = &winminheight
let s:save_winminwidth = &winminwidth
set winminheight=0
set winheight=1
set winminwidth=0
set winwidth=1
argglobal
balt rustbee-gui/src/components/header.svelte
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
let s:l = 42 - ((27 * winheight(0) + 28) / 56)
if s:l < 1 | let s:l = 1 | endif
keepjumps exe s:l
normal! zt
keepjumps 42
normal! 025|
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
nohlsearch
doautoall SessionLoadPost
unlet SessionLoad
" vim: set ft=vim :
