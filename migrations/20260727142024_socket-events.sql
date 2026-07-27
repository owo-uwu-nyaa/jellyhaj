-- Add migration script here

create table jellyfin_socket_events (
       id integer primary key,
       added integer not null default (unixepoch()),
       val text not null
) strict;
