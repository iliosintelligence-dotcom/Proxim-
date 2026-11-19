/*
  Pip, drawn smooth. A bold map-pin body with a face, built from the same
  heavy, rounded geometry as Momo Trust Display so the mascot, the logo
  and the headlines read as one family. 64 x 80 viewBox; the point of the
  pin sits at (32, 76).
*/

export const PIP_BODY =
  "M32 4C47.46 4 60 16.3 60 31.5C60 48.6 42.3 62.4 34.4 74.1a2.9 2.9 0 0 1-4.8 0C21.7 62.4 4 48.6 4 31.5 4 16.3 16.54 4 32 4Z";

export const PIP_BLUE = "#0054f9";

/** Static Pip: used for the logo, favicon and anywhere it doesn't move. */
export function PipMark({
  className = "",
  face = true,
  cheeks = true,
  color = PIP_BLUE,
}: {
  className?: string;
  /** Drop the face at tiny sizes if needed. */
  face?: boolean;
  cheeks?: boolean;
  color?: string;
}) {
  return (
    <svg viewBox="0 0 64 80" className={className} aria-hidden="true">
      <path d={PIP_BODY} fill={color} />
      <ellipse cx="20" cy="17" rx="7" ry="4.5" fill="#fff" opacity=".22" transform="rotate(-30 20 17)" />
      {face && (
        <>
          <ellipse cx="23" cy="30" rx="6.6" ry="8.2" fill="#fff" />
          <ellipse cx="41" cy="30" rx="6.6" ry="8.2" fill="#fff" />
          <circle cx="24.4" cy="31.6" r="3.6" fill="#0d0d0d" />
          <circle cx="42.4" cy="31.6" r="3.6" fill="#0d0d0d" />
          <circle cx="25.6" cy="30.2" r="1.1" fill="#fff" />
          <circle cx="43.6" cy="30.2" r="1.1" fill="#fff" />
          {cheeks && (
            <>
              <ellipse cx="15.5" cy="41" rx="3.6" ry="2.3" fill="#ff9ec0" />
              <ellipse cx="48.5" cy="41" rx="3.6" ry="2.3" fill="#ff9ec0" />
            </>
          )}
          <path d="M26.5 43.5q5.5 5 11 0" fill="none" stroke="#0b1b4a" strokeWidth="3" strokeLinecap="round" />
        </>
      )}
    </svg>
  );
}
