create table todos (
    id BIGINT UNSIGNED PRIMARY KEY NOT NULL AUTO_INCREMENT,
    description TEXT NOT NULL,
    done BOOLEAN NOT NULL DEFAULT FALSE,
    progress float,
    created_at datetime
);

create database if not exists fizz;
