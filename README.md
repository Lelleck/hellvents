# Hellvents

A collection of mini games which can be automatically enforced for Hell Let Loose. 
It ties into [wise](https://github.com/Lelleck/wise) to receive game events and enables a pseudo CLI.

## Features

- Ingame CLI: Start and stop your events from the ingame chat.
- Flexible: The internal architecture is straight forward, adding your own mini game is easy.
- Lightweight: No unnecessary overhead, Rust makes it fast and light. 

## Setup

### Requirements

Setting up hellvents is easy as long as you have the required dependencies. 

- The Git CLI. Install it from [here](https://git-scm.com/downloads).
- A Rust installation. Install it from [here](https://www.rust-lang.org/tools/install).
- An accessible [wise](https://github.com/Lelleck/wise) service. Your token must have raw write permissions.

Compiling your own binary is recommended as this will account for your systems architecture. 
It requires a bit of extra effort to setup the correct environment but allows you to much more easily update.

### Execution

First you have to clone the source code. 
To do this navigate to the parent folder where you want the project to be and execute:  
`git clone https://github.com/Lelleck/hellvents.git`  
This will download the source code to your local file system.

Should you ever need to update execute, `git pull`. This will update your local source code to the latest version.

After downloading the source code the time has come to compile and run it.
This is all done in one command and requires you to have filled out the configuration.
Refer to [Configuration Setup](#configuration-setup) for this.
Assuming you have rustup installed, [wise](https://github.com/Lelleck/wise) running and the configuration setup run:
`cargo run --release -- config.toml`  

### Configuration Setup

To prevent conflicts when updating the source code first make a copy of `config.example.toml` and preferrably name it `config.toml`.
Bear in mind that between versions the layout of the config file may change and allow you to configure new values. 
Using an outdated configuration file will lead to a crash. 
After every update cross check with the new `config.example.toml` and amend your `config.toml` accordingly.

Refer to the config file for which values to set in what manner.
