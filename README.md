# sfshare
A simple file sharing utility written in Rust


# Protocol

## PING (client -> server)
Asks server if he's active
### Data send
`[1 byte flag PING]`

## PONG (server -> client)
Response to a ping req
### Data send
`[1 byte flag PONG]`

## ACK_REQ (client -> server)
Asks if server wants to receive file(s)
Transmitted as list of `[4 byte file-id][8 byte file-size][2 byte name_len][name utf8]`
### Data send
`[1 byte flag ACK_REQ][4 byte length of list][string (list)]`