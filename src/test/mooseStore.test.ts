import { describe, it, expect, beforeEach } from "vitest";
import { useMooseStore } from "../stores/mooseStore";

describe("mooseStore State Management", () => {
  beforeEach(() => {
    useMooseStore.setState({
      characterState: "idle",
      mouthShape: "closed",
      isMuted: false,
      inputLevel: 0,
      outputLevel: 0,
    });
  });

  it("updates character state correctly", () => {
    const store = useMooseStore.getState();
    store.setCharacterState("listening");
    expect(useMooseStore.getState().characterState).toBe("listening");
  });

  it("updates mouth shapes and levels", () => {
    const store = useMooseStore.getState();
    store.setMouthShape("wide");
    store.setInputLevel(0.75);
    store.setOutputLevel(0.9);

    expect(useMooseStore.getState().mouthShape).toBe("wide");
    expect(useMooseStore.getState().inputLevel).toBe(0.75);
    expect(useMooseStore.getState().outputLevel).toBe(0.9);
  });
});
