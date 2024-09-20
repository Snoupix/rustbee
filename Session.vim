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
badd +60 README.md
badd +163 rustbee
badd +1 .gitignore
badd +1180 rustbee-gui/src/main.rs
badd +11 Cargo.toml
badd +6 rustbee-common/src/lib.rs
badd +34 rustbee-common/src/constants.rs
badd +27 src/main.rs
badd +1 rustbee-daemon/src/main.rs
badd +130 rustbee-common/src/colors.rs
badd +597 rustbee-common/src/bluetooth.rs
badd +12 rustbee-gui/Cargo.toml
badd +69 src/cli.rs
badd +1 rustbee-common/Cargo.toml
badd +18 rustbee-common/src/tests.rs
badd +198 ~/.cargo/registry/src/index.crates.io-6f17d22bba15001f/btleplug-0.11.5/src/api/bdaddr.rs
argglobal
%argdel
edit rustbee-common/src/bluetooth.rs
argglobal
balt ~/.cargo/registry/src/index.crates.io-6f17d22bba15001f/btleplug-0.11.5/src/api/bdaddr.rs
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
let s:l = 582 - ((30 * winheight(0) + 27) / 55)
if s:l < 1 | let s:l = 1 | endif
keepjumps exe s:l
normal! zt
keepjumps 582
normal! 054|
lcd ~/work/rustbee
tabnext 1
if exists('s:wipebuf') && len(win_findbuf(s:wipebuf)) == 0 && getbufvar(s:wipebuf, '&buftype') isnot# 'terminal'
  silent exe 'bwipe ' . s:wipebuf
endif
unlet! s:wipebuf
set winheight=1 winwidth=20
let &shortmess = s:shortmess_save
let s:sx = expand("<sfile>:p:r")."x.vim"
if filereadable(s:sx)
  exe "source " . fnameescape(s:sx)
endif
let &g:so = s:so_save | let &g:siso = s:siso_save
doautoall SessionLoadPost
unlet SessionLoad
" vim: set ft=vim :
