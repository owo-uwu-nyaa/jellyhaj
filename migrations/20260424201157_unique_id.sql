-- Add migration script here
create table unique_id(
       id blob not null,
       row_id integer not null primary key default 1,
       unique (row_id) on conflict replace
) strict;
