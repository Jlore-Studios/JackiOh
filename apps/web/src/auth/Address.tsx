// An email address allowed to wrap before its "@", so a long one in a narrow column breaks between its
// halves (a half too long alone breaks inside: `overflow-wrap: anywhere`, auth/tavern.css). Only a
// <wbr>: no character is added, so the text an element reads (and a test asserts) is the address exactly.

import type { ReactElement } from "react";

export default function Address({ value }: { value: string }): ReactElement {
  const at = value.lastIndexOf("@");
  if (at <= 0) return <>{value}</>;
  return (
    <>
      {value.slice(0, at)}
      <wbr />
      {value.slice(at)}
    </>
  );
}
