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
badd +119 README.md
badd +100 rustbee
badd +1 .gitignore
badd +261 rustbee-gui/src/main.rs
badd +9 Cargo.toml
badd +7 rustbee-common/src/lib.rs
badd +23 rustbee-common/src/constants.rs
badd +60 src/main.rs
badd +89 rustbee-daemon/src/main.rs
badd +130 rustbee-common/src/colors.rs
badd +22 rustbee-common/src/bluetooth.rs
badd +11 rustbee-gui/Cargo.toml
badd +127 src/cli.rs
badd +23 rustbee-common/Cargo.toml
badd +18 rustbee-common/src/tests.rs
badd +33 Justfile
badd +8 rustbee-gui/Justfile
badd +8 rustbee-daemon/Justfile
badd +29 rustbee-common/src/daemon.rs
badd +10 rustbee-daemon/Cargo.toml
badd +8 rustbee-common/librustbee.h
badd +56 rustbee-common/src/ffi.rs
badd +73 rustbee-common/src/storage.rs
badd +9 src/address.rs
badd +90 rustbee-common/src/logger.rs
argglobal
%argdel
edit README.md
wincmd t
let s:save_winminheight = &winminheight
let s:save_winminwidth = &winminwidth
set winminheight=0
set winheight=1
set winminwidth=0
set winwidth=1
argglobal
balt src/main.rs
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
let s:l = 88 - ((27 * winheight(0) + 28) / 56)
if s:l < 1 | let s:l = 1 | endif
keepjumps exe s:l
normal! zt
keepjumps 88
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
nohlsearch
doautoall SessionLoadPost
unlet SessionLoad
" vim: set ft=vim :
