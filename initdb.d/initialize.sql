create table todos (
    id int,
    description varchar(255),
    done int,
    progress float,
    created_at datetime
);

create database if not exists fizz;
