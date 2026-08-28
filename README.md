# EGRESS — The bot that spies on me (sort of)
It's really best you don't ask...

... Okay, fine. Long story short in my current housing situation if I don't
tell someone when I leave I risk getting kicked out even though there's
already cameras and alarms that does that anyway.

I hate texting people to "let them know" pointless shit, and this is the
epitome of pointless shit. So I made this. It runs on my server,
[`tokoro`](https://github.com/4jamesccraven/dotfiles/blob/main/hosts/tokoro.nix),
and whenever I send it an http request it sends messages in a Telegram chat.

Malicious compliance and all that.

## Actually Interesting Tech Stuff
I used this as an excuse to try a couple of new technologies:
- Daemon/Client architecture
- Unix sockets
- HTTP Server
- Probably other stuff but I forgor 💀

## Installation Notes (for my personal recollection)
1. Add it to system flake
2. Import NixOS module to relevant host (`inputs.egress.nixosModules.default`)
3. Set `services.egress.enable = true;`, add your user to the "egress" group.
5. Open port `50925`
4. Write a config to `/etc/egress/config.toml` (JSON supported to in case I need it)

## License Info
Since I have to release this publicly to play nice with my flake, it has a license. It's
GPL so that means you can't copy it without keeping the license. See the full license
for details, the language here is without exception superseded by that file.

License does not apply to the images under livery.

## LLM Usage
I consulted an LLM for design decisions and to generate the images under ./livery.
Everything else (especially this README if you couldn't already tell). Was handwritten.
