import App from "./App";
import { mountReactApp } from "../../shared/react-root";
import "./styles.css";

mountReactApp(document.getElementById("root"), <App />, { title: "JMCP Web Proof Host" });
