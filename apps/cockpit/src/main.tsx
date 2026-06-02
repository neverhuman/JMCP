import App from "./App";
import VoiceAssistant from "./components/VoiceAssistant";
import { mountReactApp } from "../../shared/react-root";
import "./styles.css";

// The voice assistant mounts only on the standalone cockpit entry — never inside
// the shared <App/> that the web proof-host screenshots, so visual tests are unaffected.
mountReactApp(
  document.getElementById("root"),
  <>
    <App />
    <VoiceAssistant />
  </>,
  { title: "JMCP Cockpit" },
);
