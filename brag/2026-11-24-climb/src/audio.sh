#!/bin/bash
# The mixes, epic-1 (D minor, 100 bpm: a beat 0.6 s, a bar 2.4 s) from 0 s on the beat grid.
# mix-land.wav: bars 1–8 (0–19.2 s), fading over the last beat. mix-port.wav: bars 1–4 then 7–8 (A section's
# i VI III VII | iv V+), 14.4 s, ending on V+ so the loop resolves into the opening i.
# Effects: a soft swish peaking on Jlorious's beat and on the card back's flip, and a faint D-minor chime
# (D5, A5, D6) under Jlorious, about 7 dB under the music.
set -e
cd "$(dirname "$0")"
FF=$(cat ffpath.txt)
MUSIC=../../apps/web/public/audio/music/epic-1.m4a
SR=48000
"$FF" -y -loglevel error -f lavfi -i "aevalsrc='(random(0)*2-1)*if(lt(t,0.12),pow(t/0.12,2),exp(-(t-0.12)*13))*(1-0.6*t/0.42)|(random(1)*2-1)*if(lt(t,0.12),pow(t/0.12,2),exp(-(t-0.12)*13))*(0.4+0.6*t/0.42)':s=$SR:d=0.42" \
  -af "highpass=f=700,lowpass=f=5200,aecho=0.7:0.35:38|71:0.22|0.12" swish.wav
"$FF" -y -loglevel error -f lavfi -i "aevalsrc='(0.55*sin(2*PI*587.33*t)+0.35*sin(2*PI*880*t)+0.25*sin(2*PI*1174.66*t))*min(1,t/0.01)*exp(-t*2.2)|(0.5*sin(2*PI*587.33*t)+0.38*sin(2*PI*880*t)+0.28*sin(2*PI*1174.66*t))*min(1,t/0.01)*exp(-t*2.2)':s=$SR:d=2.0" chime.wav
SW=${SW_VOL:-2.4}; CH=${CH_VOL:-0.09}
fx() { # fx <music chain> <jlorious s> <flip s> <dur> <out>
  local jl=$(python3 -c "print(int(($2-0.12)*1000))") fl=$(python3 -c "print(int(($3-0.12)*1000))") ch=$(python3 -c "print(int($2*1000))")
  "$FF" -y -loglevel error -i "$MUSIC" -i swish.wav -i swish.wav -i chime.wav -filter_complex "
    $1[m];
    [1:a]volume=$SW,adelay=$jl|$jl,aformat=sample_fmts=fltp:channel_layouts=stereo[s1];
    [2:a]volume=$SW,adelay=$fl|$fl,aformat=sample_fmts=fltp:channel_layouts=stereo[s2];
    [3:a]volume=$CH,adelay=$ch|$ch,aformat=sample_fmts=fltp:channel_layouts=stereo[c];
    [m][s1][s2][c]amix=inputs=4:normalize=0:duration=first,alimiter=limit=0.89:level=false,apad=whole_dur=$4,atrim=0:$4[out]" \
    -map "[out]" -ar $SR -c:a pcm_s16le "$5"
}
LAND="[0:a]atrim=0:19.2,asetpts=PTS-STARTPTS,aresample=$SR,aformat=sample_fmts=fltp:channel_layouts=stereo,afade=t=out:st=18.6:d=0.6"
fx "$LAND" 12.0 14.4 19.2 mix-land.wav
PORT="[0:a]asplit=2[a][b];[a]atrim=0:9.62,asetpts=PTS-STARTPTS[a1];[b]atrim=14.38:19.2,asetpts=PTS-STARTPTS[b1];[a1][b1]acrossfade=d=0.04:c1=tri:c2=tri,aresample=$SR,aformat=sample_fmts=fltp:channel_layouts=stereo,afade=t=in:d=0.01,afade=t=out:st=14.37:d=0.03"
fx "$PORT" 9.6 12.0 14.4 mix-port.wav
echo "mixes written"
