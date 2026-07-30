-- Add migration script here
drop table creds;
create table creds(
       access_token text not null,
       added integer not null default (unixepoch()),
       server_id text not null primary key,
       unique (server_id) on conflict replace
) strict;
