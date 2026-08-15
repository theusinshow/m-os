import { ImageResponse } from "next/og";

export const size = { width: 180, height: 180 };
export const contentType = "image/png";

// iOS home-screen icon. Drawn with a CSS triangle (Satori has no path support)
// to echo the brand TriangleMark on the dark base.
export default function AppleIcon() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          background: "#020A06",
        }}
      >
        <div
          style={{
            width: 0,
            height: 0,
            borderLeft: "52px solid transparent",
            borderRight: "52px solid transparent",
            borderBottom: "92px solid #FB3640",
          }}
        />
      </div>
    ),
    { ...size },
  );
}
