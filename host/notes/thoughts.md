: mstar if star star star star else star then star ;

becomes

if star star star star else star then star

becomes


0: if       => CondJump(false, +6)
1: star     => Compiled
2: star     => Compiled
3: star     => Compiled
4: star     => Compiled
5: else     => UncondJump(+1)
6: star     => Compiled         <- cond jump
7: then     => Nothing?         <- uncond*
8: star     => Compiled


0: if       => CondJump(false, +5)
1: star     => Compiled
2: star     => Compiled
3: star     => Compiled
4: star     => Compiled
7: then     => Nothing?         <- uncond*
8: star     => Compiled
