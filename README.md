Alternative to [rust-id3](https://github.com/polyfloyd/rust-id3) that does not read all frames consistently, particularly the Mixed in Key `EnergyLevel` field.

- [ ] https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-frames.html#general-encapsulated-object

application/vnd.rekordbox.dat

Reference documentation

ID3 v2.3 is UTF-16, v2.4 heeft ook UTF-8. Oude hardware verwacht soms ID3v2.3 met UTF-16LE.
[Unsynchronization can only be applied to the entire tag in 2.3, whereas you can apply it to individual frames in 2.4. This means that in 2.3, the tag size field is stored as syncsafe, while the frame sizes aren't. In 2.4 all sizes are stored as syncsafe.](https://hydrogenaud.io/index.php?topic=67145.msg602042#msg602042)

* [ID3 tag version 2.3.0](https://id3.org/id3v2.3.0)
* [ID3 tag version 2.4.0](https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-structure.html)
* [Internal structure](https://www.the-roberts-family.net/metadata/mp3.html)
* [MP3](http://www.datavoyage.com/mpgscript/mpeghdr.htm)
* https://en.wikipedia.org/wiki/MP3#/media/File:Mp3filestructure.svg

Voor MP3 audio frames gewoon op zoek gaan naar 0xFFFE

https://id3.org/id3v2.4.0-structure

```
 ┌───────────────────────┐
  ┌─────────────────────┐
   Header
   ID3...size
  ┌─────────────────────┐
   Tag
    ┌──────────────────┐
     Frame
     FRIDsize..
     ┌────────────────┐
      Data
    ┌──────────────────┐
     Frame
     FRIDsize..
     ┌────────────────┐
      Data
    ┌──────────────────┐
     Padding
  ┌─────────────────────┐
   Audio
    ┌──────────────────┐
     Frame
    ┌──────────────────┐
     Frame
    ┌──────────────────┐
     Frame
 └───────────────────────┘
```

Tests moeten niet [parallel](https://doc.rust-lang.org/book/ch11-02-running-tests.html) lopen, omdat ze dezelfde bestanden overschijven.

```shell
cargo test -- --test-threads=1
```

### Ring buffer

Telkens 1K lezen en een subset minus frame header doorzoeken, daarna seek - frame header en volgende 1K lezen.