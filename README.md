# Classic Addon Manager
Support development of this project by buying me a ko-fi.

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/X8X219OKGE)

<img align="center" src="https://github.com/user-attachments/assets/a0fc4c09-5ecc-4b80-bae0-b77e6c93cb51" width="700">

<img align="center" src="https://github.com/user-attachments/assets/eb4c6a8c-e9a5-43c0-80f1-01841599bf2e" width="700">


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
