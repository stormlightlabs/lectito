# Migrate to Material Design 3

[https://gitlab.com/fdroid/fdroidclient/-/merge\_requests/1350](https://gitlab.com/fdroid/fdroidclient/-/merge_requests/1350)

## Description

## [](#goals)Goals

*   upgrade the color system without using dynamic color
*   upgrade basic components (FAB, bottom app bar, etc.)
*   does not introduce breakage
*   minimal changes

## [](#colors)Colors

I used our current primary (fdroid\_blue `#1976d2`) and secondary color (fdroid\_green `#8ab000`) with the [Material Theme Builder](https://www.figma.com/community/plugin/1034969338659738588/material-theme-builder) (with matched color), and here is what I get: [https://www.figma.com/file/ABnrhL6k5yDa67nOchezuD/Material-3-Design-Kit-(Community)?type=design&node-id=49823%3A12141&mode=design&t=LNzqJd5skWrE7WCK-1](https://www.figma.com/file/ABnrhL6k5yDa67nOchezuD/Material-3-Design-Kit-\(Community\)?type=design&node-id=49823%3A12141&mode=design&t=LNzqJd5skWrE7WCK-1)

[![image](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/13671d2d1786674c1d00410aadcaa8dc/image.png)

We don't have a tertiary brand color. So I just used the generated tertiary color by the Material Theme Builder (magenta `#A685B1`), based on the Primary Color fdroid\_blue `#1976d2`, as [recommended](https://m3.material.io/foundations/customization#b4bea241-95cb-4e46-9668-ddd37c760955) by the guidelines for this situation.

[![image](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/5bee9ea62d76cf064311b6b256c9d8cd/image.png)

[![image](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/b808d4ba590bcbe4bdd5e98bcf9f2933/image.png)

## [](#preview)Preview

### [](#light)Light

[![image](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/3420ba75fb4d9467a3d003c0be9f52da/image.png)[![28f623a8-2eb6-4a5d-89d8-16acf231baf6](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/f7d86e660492598e9446769067d0f385/28f623a8-2eb6-4a5d-89d8-16acf231baf6.png)[![81744797-83bf-4bcf-b9a5-081579e6a8fc](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/15abd2aaf46391edb1b4ec09b9e44f10/81744797-83bf-4bcf-b9a5-081579e6a8fc.png)[![image.png](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/337e9f83e1cfcd1f6f1aa1b9f347fbcd/image.png)[![image](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/5563a32ce3a8226852970f396be839ad/image.png)[![image](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/9a11c8bbdee1122fcbe651167ea553df/image.png)[![image](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/cecafda5ccfb95aedcbafe1bfe7c4e22/image.png)[![image.png](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/b42c74efaf49bf37a50a73e0d20af92b/image.png)

### [](#dark)Dark

[![2a4d7fe3-c6f2-4e44-92f9-7b8ae7a0c713](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/1c3aa56f8076ca93855364b2f92c9ce4/2a4d7fe3-c6f2-4e44-92f9-7b8ae7a0c713.png)[![4c9a288f-4936-48d4-b547-035e8403fad1](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/9c2c5e444ae1cb56b1f78b73526305cf/4c9a288f-4936-48d4-b547-035e8403fad1.png)[![a61f50a7-76ed-486c-b191-8212a376468e](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/6bf1cb0a77f680bebae67f70e927ca30/a61f50a7-76ed-486c-b191-8212a376468e.png)[![image](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/7780036d6ce44d38c341168c0219d2cf/image.png)[![image.png](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/dc098ec1f15bb097a22a06557ea33736/image.png)[![image.png](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/c4f9f42a53b98beafa2ce2b5df2e7212/image.png)[![055b6bca-d267-45d5-abe8-04419b26dac1.png](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/6ce9667781d6d0193b293abda5bb5c85/055b6bca-d267-45d5-abe8-04419b26dac1.png)[![2123e4ef-01fd-47fb-820c-5c2b694362c8.png](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/22eb8f1a9c6a6b475ac1b12ce4be0942/2123e4ef-01fd-47fb-820c-5c2b694362c8.png)

### [](#black)Black

[![121f881f-0400-43f8-84c1-64a3a0cfa582](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/3a1f32f13d353ac49c26a5db80fc2e1c/121f881f-0400-43f8-84c1-64a3a0cfa582.png)[![a209b5d7-13c6-44a2-b324-b8a72c09bbad](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/4e5cf9caac71039a13adedfcb882da72/a209b5d7-13c6-44a2-b324-b8a72c09bbad.png)[![cf283e16-5ad0-4685-be52-cadc15016810](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/2b22cfe0c6f361d607fc1ab923e9a6bd/cf283e16-5ad0-4685-be52-cadc15016810.png)[![image](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/7b6083f618a227ef072481e8e2c07212/image.png)[![image.png](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/46a38ab31ade5af8b5f214b30fe6445d/image.png)[![image](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/c0ae86c893023d5d4df9a6dead60464e/image.png)[![698ebf16-9a70-4868-8732-a58af364ae86.png](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/0cf85bed0fa6c2d9fe7664e0c995a2a7/698ebf16-9a70-4868-8732-a58af364ae86.png)[![4166ce81-9617-4225-b206-f3e45c8bdd8b.png](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)](/-/project/36189/uploads/01f66d208c2193b9c84f68cb86495139/4166ce81-9617-4225-b206-f3e45c8bdd8b.png)

Fixes [#1962 (closed)](/fdroid/fdroidclient/-/issues/1962 "New efficient Material Design for the App"), [#2246 (closed)](/fdroid/fdroidclient/-/issues/2246 "(Dark theme) Contrast between elements is too high after 1.13 theme changes"), [#2511 (closed)](/fdroid/fdroidclient/-/issues/2511 "Support Material 3/You."), [#2927 (closed)](/fdroid/fdroidclient/-/issues/2927 "Cosmetic: bell icon in Updates screen doesn't look consistent between dark and light mode"), [#2926 (closed)](/fdroid/fdroidclient/-/issues/2926 "Cosmetic: featureGraphic is high up and cut off")