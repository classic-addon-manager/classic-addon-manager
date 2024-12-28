# Classic Addon Manager
<img align="center" src="https://github.com/user-attachments/assets/0804a15e-4f6e-4a0a-9400-c8aa73029a4c" width="700">
<img align="center" src="https://github.com/user-attachments/assets/593b5b46-8cb7-4858-9764-2949968e699c" width="700">

## Development requirements
You need to install the following dependencies for Wails to be operational.
    Go 1.20+
    NPM (Node 15+)
Once that is done visit the following link to install the Wails CLI

https://wails.io/docs/gettingstarted/installation

## Live Development

To run in live development mode, run `wails dev` in the project directory. This will run a Vite development
server that will provide very fast hot reload of your frontend changes. If you want to develop in a browser
and have access to your Go methods, there is also a dev server that runs on http://localhost:34115. Connect
to this in your browser, and you can call your Go code from devtools.

## Building

To build a redistributable, production mode package, use `wails build`.
