#!/usr/bin/env nu

def "main json-default-config" [] {
  cargo run -- --print-default-json-config | save -f "assets/config/default.json"
}

def main [] { }
