# This is impossible to read, but these encodings rarely if ever change, so
# it probably does not matter. They are derived from the list given here:
# https://encoding.spec.whatwg.org/#concept-encoding-get
#
# The globbing here works in both fish and zsh (though they expand it in
# different orders). It may work in other shells too.

{{,us-}ascii,arabic,chinese,cyrillic,greek{,8},hebrew,korean}
logical visual mac {,cs}macintosh x-mac-{cyrillic,roman,ukrainian}
866 ibm{819,866} csibm866
big5{,-hkscs} {cn-,cs}big5 x-x-big5
cp{819,866,125{0,1,2,3,4,5,6,7,8}} x-cp125{0,1,2,3,4,5,6,7,8}
csiso2022{jp,kr} csiso8859{6,8}{e,i}
csisolatin{1,2,3,4,5,6,9} csisolatin{arabic,cyrillic,greek,hebrew}
ecma-{114,118} asmo-708 elot_928 sun_eu_greek
euc-{jp,kr} x-euc-jp cseuckr cseucpkdfmtjapanese
{,x-}gbk csiso58gb231280 gb18030 {,cs}gb2312 gb_2312{,-80} hz-gb-2312
iso-2022-{cn,cn-ext,jp,kr}
iso8859{,-}{1,2,3,4,5,6,7,8,9,10,11,13,14,15}
iso-8859-{1,2,3,4,5,6,7,8,9,10,11,{6,8}-{e,i},13,14,15,16} iso_8859-{1,2,3,4,5,6,7,8,9,15}
iso_8859-{1,2,6,7}:1987 iso_8859-{3,4,5,8}:1988 iso_8859-9:1989
iso-ir-{58,100,101,109,110,126,127,138,144,148,149,157}
koi{,8,8-r,8-ru,8-u,8_r} cskoi8r
ks_c_5601-{1987,1989} ksc{,_}5691 csksc56011987
latin{1,2,3,4,5,6} l{1,2,3,4,5,6,9}
shift{-,_}jis csshiftjis {,x-}sjis ms_kanji ms932
utf{,-}8 utf-16{,be,le} unicode-1-1-utf-8
windows-{31j,874,949,125{0,1,2,3,4,5,6,7,8}} dos-874 tis-620 ansi_x3.4-1968
x-user-defined auto none
