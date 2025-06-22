# Classic Addon Manager
Support development of this project by buying me a ko-fi.

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/X8X219OKGE)


<img align="center" src="https://github.com/user-attachments/assets/8f521ae7-cf14-4915-a58e-6a6d074d74f1" width="700">

<img align="center" src="https://github.com/user-attachments/assets/f0243db8-8dc8-4c00-b244-00a32429b31c" width="700">

<img align="center" src="https://github.com/user-attachments/assets/3d3e21e1-aad8-43e8-856f-da112b8d3da2" width="700">

<img align="center" src="https://github.com/user-attachments/assets/5b081507-ddfa-4128-8ced-b104766b7d6e" width="700">


## Development requirements
You need to install the following dependencies for Wails to be operational.
    Go 1.24+
    NPM (Node 15+)
Once that is done visit the following link to install the Wails CLI

https://v3alpha.wails.io/getting-started/installatio

_It's important to note that we are using the Wails v3 alpha version, please make sure you are installing the correct version._
## Live Development

To run in live development mode, run `wails3 dev` in the project directory. This will run a Vite development
server that will provide very fast hot reload of your frontend changes. If you want to develop in a browser
and have access to your Go methods, there is also a dev server that runs on http://localhost:34115. Connect
to this in your browser, and you can call your Go code from devtools.

## Building

To build a redistributable, production mode package, use `wails3 task build:prod`.
