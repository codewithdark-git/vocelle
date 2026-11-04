import { AirGap } from "@/components/AirGap";
import { Anywhere } from "@/components/Anywhere";
import { Closing } from "@/components/Closing";
import { Download } from "@/components/Download";
import { Hero } from "@/components/Hero";
import { Product } from "@/components/Product";
import { Race } from "@/components/Race";
import { SmoothScroll } from "@/components/SmoothScroll";
import { Specs } from "@/components/Specs";
import { WordRibbon } from "@/components/WordRibbon";

export default function Home() {
  return (
    <main>
      <SmoothScroll />
      <Hero />
      <WordRibbon />
      <Anywhere />
      <Product />
      <Race />
      <AirGap />
      <Specs />
      <Download />
      <Closing />
    </main>
  );
}
