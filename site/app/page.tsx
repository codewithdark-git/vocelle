import { AirGap } from "@/components/AirGap";
import { Anywhere } from "@/components/Anywhere";
import { Closing } from "@/components/Closing";
import { Grain, HeadingReveal, StickyBar } from "@/components/Craft";
import { Download } from "@/components/Download";
import { Cost, Limits } from "@/components/Facts";
import { Hero } from "@/components/Hero";
import { Race } from "@/components/Race";
import { SmoothScroll } from "@/components/SmoothScroll";
import { Specs } from "@/components/Specs";
import { Tour } from "@/components/Tour";
import { WordRibbon } from "@/components/WordRibbon";

export default function Home() {
  return (
    <>
      <SmoothScroll />
      <HeadingReveal />
      <StickyBar />
      <Grain />
      <main>
        <Hero />
        <WordRibbon />
        <Tour />
        <Anywhere />
        <AirGap />
        <Cost />
        <Race />
        <Specs />
        <Limits />
        <Download />
        <Closing />
      </main>
    </>
  );
}
