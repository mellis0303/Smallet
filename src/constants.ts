import { buildCoderMap } from "@saberhq/anchor-contrib";
import { PublicKey } from "@solana/web3.js";

import { SmalletJSON } from "./idls/smallet";
import type { SmalletProgram, SmalletTypes } from "./programs";

export interface Programs {
  Smallet: SmalletProgram;
}

export const COSMIC_ADDRESSES = {
  Smallet: new PublicKey("7vZw152zk65W5F59S6Svt29JjDhzZocWvF7PYvjNCAKB"),
};

export const COSMIC_IDLS = {
  Smallet: SmalletJSON,
};

export const COSMIC_CODERS = buildCoderMap<{
  Smallet: SmalletTypes;
}>(COSMIC_IDLS, COSMIC_ADDRESSES);
