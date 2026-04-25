-- Add migration script here
create table creds(
       user_name text not null,
       access_token text not null,
       added integer not null default (unixepoch()),
       row_id integer not null primary key default 1,
       unique (row_id) on conflict replace
) strict;
