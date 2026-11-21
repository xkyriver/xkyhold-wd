#!/bin/bash
# Helpers prepended to every probe: click(text), fill(selector, value), text().
H='const click=(t)=>{const b=[...document.querySelectorAll("button,label")].find(e=>e.innerText.trim().startsWith(t));if(!b)return "NO "+t;b.click();return "clicked "+t};const fill=(s,v)=>{const e=document.querySelector(s);if(!e)return "NO "+s;e.value=v;e.dispatchEvent(new Event("input",{bubbles:true}));e.dispatchEvent(new Event("change",{bubbles:true}));return "filled "+s};const text=()=>document.querySelector("main").innerText;'
"$(dirname "$0")/probe.sh" "$H $1" "${2:-30}"
