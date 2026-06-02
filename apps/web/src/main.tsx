import App from "./App";
import { mountReactApp } from "../../shared/react-root";
import "./design-tokens.css";
import "./styles.css";

mountReactApp(document.getElementById("root"), <App />, { title: "JMCP Web Proof Host" });
