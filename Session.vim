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
badd +74 README.md
badd +100 rustbee
badd +1 .gitignore
badd +1382 rustbee-gui/src/main.rs
badd +9 Cargo.toml
badd +17 rustbee-common/src/lib.rs
badd +53 rustbee-common/src/constants.rs
badd +48 src/main.rs
badd +116 rustbee-daemon/src/main.rs
badd +130 rustbee-common/src/colors.rs
badd +545 rustbee-common/src/bluetooth.rs
badd +13 rustbee-gui/Cargo.toml
badd +351 src/cli.rs
badd +10 rustbee-common/Cargo.toml
badd +18 rustbee-common/src/tests.rs
badd +66 Justfile
badd +8 rustbee-gui/Justfile
badd +8 rustbee-daemon/Justfile
badd +29 rustbee-common/src/daemon.rs
badd +1 rustbee-common/src/logs.rs
badd +7 rustbee-daemon/Cargo.toml
badd +9 rustbee-common/librustbee.h
badd +56 rustbee-common/src/ffi.rs
argglobal
%argdel
edit src/cli.rs
let s:save_splitbelow = &splitbelow
let s:save_splitright = &splitright
set splitbelow splitright
wincmd _ | wincmd |
vsplit
1wincmd h
wincmd w
let &splitbelow = s:save_splitbelow
let &splitright = s:save_splitright
wincmd t
let s:save_winminheight = &winminheight
let s:save_winminwidth = &winminwidth
set winminheight=0
set winheight=1
set winminwidth=0
set winwidth=1
wincmd =
argglobal
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
let s:l = 345 - ((27 * winheight(0) + 27) / 55)
if s:l < 1 | let s:l = 1 | endif
keepjumps exe s:l
normal! zt
keepjumps 345
normal! 037|
lcd ~/work/rustbee
wincmd w
argglobal
if bufexists(fnamemodify("~/work/rustbee/rustbee-common/librustbee.h", ":p")) | buffer ~/work/rustbee/rustbee-common/librustbee.h | else | edit ~/work/rustbee/rustbee-common/librustbee.h | endif
if &buftype ==# 'terminal'
  silent file ~/work/rustbee/rustbee-common/librustbee.h
endif
balt ~/work/rustbee/rustbee-common/src/ffi.rs
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
let s:l = 8 - ((7 * winheight(0) + 27) / 55)
if s:l < 1 | let s:l = 1 | endif
keepjumps exe s:l
normal! zt
keepjumps 8
normal! 0
lcd ~/work/rustbee
wincmd w
wincmd =
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
