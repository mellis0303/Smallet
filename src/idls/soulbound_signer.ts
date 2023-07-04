import { generateErrorMap } from "@saberhq/anchor-contrib";

export type SoulboundSignerIDL = {
  version: "0.11.1";
  name: "soulboundsigner";
  docs: ["Soulbound signer program"];
  instructions: [
    {
      name: "invokeSignedInstruction";
      accounts: [
        {
          name: "ownerAuthority";
          isMut: false;
          isSigner: true;
          docs: ["Authority attempting to sign."];
        },
        {
          name: "nftAccount";
          isMut: false;
          isSigner: false;
          docs: [
            "Account containing at least one token.",
            "This must belong to `owner_authority`."
          ];
        },
        {
          name: "nftPda";
          isMut: false;
          isSigner: false;
          docs: ["PDA associated with the NFT."];
          pda: {
            seeds: [
              {
                kind: "const";
                type: "string";
                value: "SoulboundSigner";
              },
              {
                kind: "account";
                type: "publicKey";
                account: "TokenAccount";
                path: "nft_account.mint";
              }
            ];
          };
        }
      ];
      args: [
        {
          name: "data";
          type: "bytes";
        }
      ];
    }
  ];
  errors: [
    {
      code: 6000;
      name: "Unauthorized";
      msg: "Unauthorized.";
    }
  ];
};
export const SoulboundSignerJSON: SoulboundSignerIDL = {
  version: "0.11.1",
  name: "soulboundsigner",
  docs: ["Soulbound signer program"],
  instructions: [
    {
      name: "invokeSignedInstruction",
      accounts: [
        {
          name: "ownerAuthority",
          isMut: false,
          isSigner: true,
          docs: ["Authority attempting to sign."],
        },
        {
          name: "nftAccount",
          isMut: false,
          isSigner: false,
          docs: [
            "Account containing at least one token.",
            "This must belong to `owner_authority`.",
          ],
        },
        {
          name: "nftPda",
          isMut: false,
          isSigner: false,
          docs: ["PDA associated with the NFT."],
          pda: {
            seeds: [
              {
                kind: "const",
                type: "string",
                value: "SoulboundSigner",
              },
              {
                kind: "account",
                type: "publicKey",
                account: "TokenAccount",
                path: "nft_account.mint",
              },
            ],
          },
        },
      ],
      args: [
        {
          name: "data",
          type: "bytes",
        },
      ],
    },
  ],
  errors: [
    {
      code: 6000,
      name: "Unauthorized",
      msg: "Unauthorized.",
    },
  ],
};
export const SoulboundSignerErrors = generateErrorMap(SoulboundSignerJSON);
