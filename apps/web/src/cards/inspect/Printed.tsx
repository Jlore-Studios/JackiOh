// The card as printed, beside a face in play that prints something else (SPEC §10.10), as #98 Heroic
// Power's eight powers beside the one it rolled. It shows only where face and print differ
// (FaceModel.printed), never for a card play keeps a mystery ("???"); it is marked as its face is
// (R277) and names its references (R279), and a Chinese card's is its Chinese text (R1301).

import type { ReactElement } from "react";
import type { FaceModel } from "../model.ts";
import { t } from "../../i18n/i18n.ts";
import { RulesText } from "../RulesText.tsx";
import { INSPECT_PRINTED } from "./testids.ts";

export function Printed({ face }: { face: FaceModel }): ReactElement | null {
  const printed = face.printed;
  if (printed === null) return null;
  return (
    <div className="inspect-printed" data-testid={INSPECT_PRINTED}>
      <p className="inspect-printed-label">{t("Printed")}</p>
      <p className="inspect-printed-text">
        <RulesText text={printed.full} marks={printed.marks} refs={face.refs} chinese={face.chinese === true} />
      </p>
    </div>
  );
}
