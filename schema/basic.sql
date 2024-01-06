create type USER_ROLE AS ENUM('admin', 'super', 'normal');

create type GROUP_TYPE as enum('channel', 'room', 'team');

create table users (
    id serial primary key,
    username text unique not null,
    "password" varchar(255) not null,
    salt text not null,
    display_name text not null,
    email text unique not null,
    created_at timestamptz default now(),
    role USER_ROLE not null default 'normal'
);

create table groups (
    id serial primary key,
    "type" GROUP_TYPE Not null default 'channel',
    group_name text not null,
    parent_group_id integer references groups(id),
    created_by integer references users(id),
    created_at timestamptz default now()
);

create table group_permissions (
    id serial primary key,
    created_by integer references users(id),
    created_at timestamptz default now(),
    group_id integer references groups(id) not null,
    user_id integer references users(id) not null,
    "read" boolean default true not null,
    write boolean default false not null,
    moderate boolean default false not null,
    -- meaning can delete other's messages + own
    "admin" boolean default false not null -- can add / remove other messages & give admin / moderate
);

create table messages (
    id serial primary key,
    sender_user_id integer references users(id) not null,
    group_id integer references groups(id) not null,
    content text not null,
    sent_at timestamptz default now()
);