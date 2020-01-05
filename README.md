# sfshare
A simple file sharing utility written in Rust
Uses TCP on port 5123
Every file has a checksum and file-id so no not-requested files can be send + you get notified if transmission failed

## Usage

First start the program in receive-mode on one pc,
then send files from sender to specific ip (should be displayed on receiver)

### Receiving
`./sfshare recv` and wait :D If sender wants to send files, accept with y

### Sending
`./sfshare send <ip> [patterns / filenames]`
Selects all files matching the pattern(s) and tries to send them. if files are large
you get asked if you really want to send those.

# Protocol

## PING (send -> recv)
Asks recv if he's active
### Data send
`[1 byte flag PING]`

## PONG (recv -> send)
Response to a ping req
### Data send
`[1 byte flag PONG]`

## ACK_REQ (send -> recv)
Asks if server wants to receive file(s)
Transmitted as list of `[4 byte file-id][8 byte file-size][2 byte name_len][name utf8]`
### Data send
`[1 byte flag ACK_REQ][4 byte length of list][string (list)]`

## ACK_RES (recv -> send)
Ackn. general file recv
### Data send
`[1 byte flag ACK_RES][1 byte bool]`

# File transmission
The receiver stores the current transmitted file meta data and handle.
Only one file can get transmitted at a time!

Per file: add all bytes to u64 counter mod 18446744073709551557 (largest unsigned 64 bit prime)

## FILE_BLOCK (send -> recv)
Standard file bytes, write to disk
### Data send
`[1 byte flag FILE_BLOCK][4 byte file id][2 byte block size n][n bytes of file data]`

## FILE_END (send -> recv)
File is finished, send checksum (no feedback wanted??)
### Data send
`[1 byte flag FILE_END][8 byte checksum]`