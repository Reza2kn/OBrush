//! Actual shared host captions; independent from reference catalogs/commands/schema.
//! No locale is admitted from its enum, a catalog lookup or a font filename.
use crate::app_preferences::{Locale, PreferenceProfile};
use crate::i18n::{TextArguments, source_text};
use crate::shortcut_profile::{Command, ShortcutResolver};
use crate::source_text_keys::SourceTextKey;
use obrush_core::BrushKind;
use std::cell::RefCell;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum CaptionKey {
    CC38AF64CE9B3,
    CADBA4F3F1C64,
    C0903E803CA38,
    CB117A63DA026,
    C10C069826A6D,
    CCC4BF142C03D,
    CF048A0D08F99,
    C630045C3B717,
    C3381DC4E7656,
    C9BD78F90D72D,
    CD282024F3CDC,
    C6E0E4B166896,
    C41CF01C0046A,
    CC7FBCF6BF3A2,
    C0FFE44A69286,
    C5D4BF2363491,
    C7CD6B496C269,
    CDEB97020792A,
    CB52A0B8FA0D0,
    C7B9FFB1D1AD0,
    C412142F1C5A4,
    C463DA7399217,
    C4544F5E90240,
    C57A4942BA374,
    CFEF18E1049D0,
    C40616030C681,
    C8B4642F04BE2,
    C1F392D24F45E,
    CD1E3F59240E2,
    C60D9EBD387D1,
    C88D8927CDFC5,
    C92BBDEC9929D,
    CCE56B4B3A72D,
    C2DB8E912D1CD,
    CAF0C53FFF14B,
    C35C9431076A9,
    C59558E3EEF38,
    C6D9383ACE6DE,
    CEF675F65EAAF,
    C1764AFB17B44,
    C87FD6BDFEF42,
    C9315CD299160,
    CAF37755AFA12,
    C7150E23974A0,
    C971BB02CB270,
    C9447B1539BDA,
    C4BAB4B66A689,
    CCCFE760A4456,
    C721F6BEF7054,
    C88AD2ABFC83B,
    C792725753B7A,
    C5C53594D8A50,
    C8F9C38536FE0,
    C170F7F889B31,
    C4E05C6BB6EB2,
    C92C0B43B9C6D,
    C9A75588A825E,
    CA903EAB2C738,
    C79723D0D02E6,
    C23398E36577D,
    CC3F4196857EC,
    CCFB20D20D92C,
    C6C6414320FCC,
    C337856576635,
    C85E2A74A55A9,
    CA6D087342259,
    C3E08F0D470B5,
    C4EBAFA696187,
    C8B0C87C10042,
    C5FB9611FCB10,
    C675745B23F75,
    C3ADB5023D4BA,
    C4DC88EFDEFB6,
    C805DC198FAA1,
    CC92989E6A92F,
    C49A6E85B293B,
    C5B8A2F217037,
    CC703AB13B28E,
    CFB3E700C4706,
    C2757CDD813C9,
    C474BF3DF97F6,
    CFDD07C4B76A6,
    CF42EFB55A3A5,
    CCE571CAD918B,
    C9BC4A75BCFFF,
    CE781304E11F4,
    C7BC3EED2F005,
    CD64CE52F6362,
    C5C593338A00C,
    C215ED96071EE,
    CED5BF4431B03,
    C19E565CBC989,
    CDA6DF21C4B10,
    C010BAA4FC183,
    C566352BE2B27,
    CA2DDC766055F,
    C57AD8D0F9C5B,
    C2886C2226F0F,
    C996E0079E48E,
    CAE33D3DD58A6,
    C2DD2245CB1A5,
    C5C7E88E89B9B,
    C7FC0C24E4A7D,
    C9FAF36F3443D,
    CED1CBE18A173,
    C0A9341892DD6,
    CE064C92D289E,
    C4CDC2FA5E56B,
    C9B45876EA4DC,
    CD3CEEFE9068F,
    C678AF70390D2,
    C31809CF81EFE,
    C97FCF490FDDC,
    CA65108620F16,
    C417A00C008A1,
    C4C39AE12DFA7,
    C9E1DEE7A566A,
    C28B3E06D0F8F,
    C98589F40BA48,
    CA0EA42396A95,
    CA7F63B73C877,
    CD3053E29DD1F,
    CDC59BA10F835,
    C1BE6D2F01842,
    CF4FB2B02B8FF,
    CFF7BC7F66552,
    C56AB4FE19F89,
    C8B737D16D7B3,
    CECAA211C172F,
    CE7F3367AA3C3,
    C28A8888DE6F5,
    CC6A4DB3EE261,
    C48AC9C2722B2,
    CAC6F86452736,
    CCF7B351FA82F,
    C9077D4A91FEF,
    C189B82C8A250,
    CC6AF1CED3192,
    CE4D9EC144301,
    CF25E5396EC0E,
    CDCA99E6F20A8,
    C058247451000,
    CB872669E1C20,
    C4D4A7F97846F,
    CA0143501678A,
    CF3975F3FCECB,
    CCCF95AFDBE90,
    CADF15C2BCBAA,
    CA67E1E58E923,
    CA17BCDB89EFD,
    C8C3745F29067,
    CB0C5D68D33CB,
    C70F1D6BE7E9A,
    C1A8DC4DCA79A,
    C729B2CEA87BC,
    C91A20036F554,
    CDB351844E20B,
    CB728B3D8AF82,
    CB3E8D2C9DE2A,
    C45F6A7866079,
    C577F3AA38DE7,
    C87B4E2BF9896,
    CD72F20ED5AB8,
    C3E0A4B565275,
    C0B17F582BE32,
    C5347433F28DE,
    CFD76CC07D179,
    C3A95F39447FA,
    C856D2A1F51BF,
    C404BB0A38ADC,
    C7C2D9064FA2A,
    CC0859E17D49E,
    C4E87CE4CDFEB,
    CF6C5A19FCE27,
    C44D8C149FF26,
    C2E9F2849C611,
    C9322339DD2B2,
    C3A77138D3BE5,
    C5C435BA8362A,
    CB13460C496E5,
    C689DA08EC6C7,
    CB8EA20FB897B,
    C7984A45F9F2C,
    C219702B49D4B,
    C8EDFE3E5A112,
    C548AB144F3B7,
    C5DC265351538,
    CF05D2176850F,
    CE802A8747436,
    C3F2A7C23CAD7,
    C7589718A5F7D,
    C6B7975DA72CA,
    C9D2300FF648F,
    C3C189345B7E1,
    CB81316F6B3A4,
    CF214501F9C03,
    C7F0D9399CB2C,
    C75A14B79611F,
    CA4277F551BC1,
    CE7C8AEB2B7AB,
    C8368312596B0,
    C3DF38CCF596B,
    C02B587E03098,
    C53040DD3EFE4,
    C93D818E52343,
    CDB684170E645,
    CA09A9445B430,
    CF3F850B891AC,
    CAC4F31BFC3D4,
    CF5602B18C2D9,
    C9ACCC7445A77,
    C22B57379AED8,
    CEF491282C4FE,
    CF91846123F5B,
    CD342B0124270,
    C48CD47AF01EA,
    C803979D8917F,
    C8309D228EC76,
    C218C4BF11852,
    C5A56A2F6F982,
    C0DB451BE86C9,
    C752CC88F4CAA,
    C707DD6DE65D6,
    C9ED18C72A105,
    C5C2AEB9985AF,
    C09B529E2E0EF,
    C0FCAE54E4CE8,
    CCD829A253D85,
    C499EEDC4E01E,
    C6BFC8C503C21,
    C5E0B2C13B602,
    C2F4AFEB04806,
    CC13311FF9FF6,
    CF5D41785D514,
    CB480949ED50E,
    CB591FA407121,
    C9370B67E7131,
    C565099F6DF99,
    C1F51DB0D93A3,
    CC5875DE51DBD,
    CB74FE24C7254,
    CD0625C68DA0D,
    CC0AAB1B128DF,
    CF3680B8703EA,
    C37416D9B13ED,
    C86096F6BA3BC,
    C1F3E8A33E7AB,
    C4B91AA94895E,
    C43E1A5DAC71D,
    C9E74CD3D8D6C,
    CCE3D63DAB0CC,
    C08547D0298E8,
    CC7B2AF4A18D1,
    C02F933521F03,
    C8A58CA483875,
    CB1768462B9D8,
    C494F5F6A28A4,
    C18DCDB07BBF0,
    C9687CB92031A,
    C045BEF52111D,
    C515ED995EE97,
    CE3B5ADDCE549,
    CCB65CD31C1AC,
    CB7CBE4272AD3,
    C4BF09CCF0C75,
    C8D63821A78EB,
    CDA6B23E1206A,
    CC94C5D60B21F,
    C749A7E33B5DB,
    C6595EA93D585,
    CAA1F976CA46F,
    C6705C63BD5AE,
    C8E657D951526,
    C724E01D8184E,
    CF6C725C6B0DF,
    CB66EBE1DC94C,
    C5F17684A2709,
    C9596375FADC7,
    C49051AAF77BE,
    C28D42BDA850A,
    C2367774907F7,
    CDB546E0641A5,
    CF4E1D9942518,
    C2D89DD5192E5,
    C3B6794E612E8,
    C28CBD374B931,
    CCA96CD519A3E,
    C339E0571E3B3,
    C1757011FD6FE,
    C0961041BAAB8,
    C54ED4988F9F3,
    C4DFABF7285EC,
    C1595029C133A,
    C826E8E985810,
    CF0BE4E07E1B5,
    C024FEA2A04D9,
    C0C60415D550D,
    C4BBB63DB3489,
    CC9F29BB8DD49,
    CEDBF3E66F400,
    CB18728F64DCE,
    CE6E53CBDE7E0,
    CD223A8379435,
    C9BC0507778FE,
    C4FF709B6A931,
    C428977F68F02,
    CE18100AD7A04,
    CF3D2EE694890,
    C388E3C24E35F,
    C5F07E4E57E8E,
    C50EC2A068A9D,
    C052EA75E8A83,
    CDD56571106B5,
    CE291C8967181,
    C732CEFA7F025,
    C995631214BEB,
    C697298D2FC48,
    C36D30A95BF87,
    C6AF7E9EDB552,
    CB0A742941259,
    C343B5567CC00,
    C1D28E6405CCE,
    CADF0F4820146,
    CF86F7FABAA92,
    C1F992AFEED38,
    C019C270B5172,
    CE6BC07E09594,
    CF094D032EAE4,
    CD2E97146E61C,
    CA3BA7BB8E3A4,
    C37B37490F9D1,
    C824E3CA3BB64,
    CB4F9B2FD35C6,
    C855D42092810,
    CF0EB81275BC4,
    C8B35DA5A3251,
    C57F53962541F,
    C61AE79146911,
    C7688CD7B2014,
    CCB106B20524D,
    C360B1EA7EA20,
    C35E9CC1A3A84,
    C08E78D02516B,
    CFF8B0C01664C,
    C5CDC021A9C30,
    CFAA8A951AE15,
    C413F1679B6B3,
    CFAF8AA87390F,
    C85F5FE9A6AF0,
    CB67CE0968F72,
    C2580AF0E9971,
    C29ED1DCAF47D,
    C31D1338A98CF,
    CF42740422670,
    CA660C279F900,
    CC1C8D65B8EDC,
    C2C8B48FE1BC1,
    CB385F53D6E13,
    C34C01497A0F9,
    CursorStyle,
    CursorStudio,
    CursorSourceBackground,
    CursorEmptyScope,
    PreventFolds,
    PreventFoldsHelp,
}
struct CaptionSpec {
    id: &'static str,
    english: &'static str,
    source: Option<SourceTextKey>,
}
const SPECS: [CaptionSpec; 365] = [
    CaptionSpec {
        id: "app.environment_panel.c38af64ce9b3",
        english: "ENVIRONMENT",
        source: Some(SourceTextKey::RenderingEnvironment),
    },
    CaptionSpec {
        id: "app.environment_panel.adba4f3f1c64",
        english: "Filmic tone mapping",
        source: Some(SourceTextKey::RenderingFilmic),
    },
    CaptionSpec {
        id: "app.environment_panel.0903e803ca38",
        english: "Darken unselected objects",
        source: Some(SourceTextKey::DarkenUnselected),
    },
    CaptionSpec {
        id: "app.environment_panel.b117a63da026",
        english: "Symmetry guide",
        source: Some(SourceTextKey::RenderingSymmetryLine),
    },
    CaptionSpec {
        id: "app.environment_panel.10c069826a6d",
        english: "Show a guide on the active part when X symmetry is enabled. This changes the view only.",
        source: None,
    },
    CaptionSpec {
        id: "app.environment_panel.cc4bf142c03d",
        english: "Reference preview uses the part’s center at zero offset. Sculpting keeps the current brush plane.",
        source: None,
    },
    CaptionSpec {
        id: "app.environment_panel.f048a0d08f99",
        english: "Custom background",
        source: None,
    },
    CaptionSpec {
        id: "app.environment_panel.630045c3b717",
        english: "Fill viewport",
        source: Some(SourceTextKey::BackgroundFill),
    },
    CaptionSpec {
        id: "app.environment_panel.3381dc4e7656",
        english: "Import background image",
        source: None,
    },
    CaptionSpec {
        id: "app.environment_panel.9bd78f90d72d",
        english: "Reset background image",
        source: Some(SourceTextKey::BackgroundReset),
    },
    CaptionSpec {
        id: "app.environment_panel.d282024f3cdc",
        english: "Blur",
        source: None,
    },
    CaptionSpec {
        id: "app.morph_panel.6e0e4b166896",
        english: "MORPH TARGET",
        source: None,
    },
    CaptionSpec {
        id: "app.morph_panel.41cf01c0046a",
        english: "Select the stored level to restore or use Morph.",
        source: None,
    },
    CaptionSpec {
        id: "app.morph_panel.c7fbcf6bf3a2",
        english: "Store a shape, then brush back toward it.",
        source: None,
    },
    CaptionSpec {
        id: "app.morph_panel.0ffe44a69286",
        english: "Replace target",
        source: None,
    },
    CaptionSpec {
        id: "app.morph_panel.5d4bf2363491",
        english: "Store target",
        source: None,
    },
    CaptionSpec {
        id: "app.morph_panel.7cd6b496c269",
        english: "Save the current visible shape at this subdivision level. Replacing the target can be undone.",
        source: None,
    },
    CaptionSpec {
        id: "app.morph_panel.deb97020792a",
        english: "Delete",
        source: None,
    },
    CaptionSpec {
        id: "app.morph_panel.b52a0b8fa0d0",
        english: "Remove the stored shape. Undo restores it.",
        source: None,
    },
    CaptionSpec {
        id: "app.morph_panel.7b9ffb1d1ad0",
        english: "Restore shape",
        source: None,
    },
    CaptionSpec {
        id: "app.morph_panel.412142f1c5a4",
        english: "Restore every point to the stored shape; masks and paint stay as they are. Undo restores the sculpt.",
        source: None,
    },
    CaptionSpec {
        id: "app.morph_panel.463da7399217",
        english: "Swap",
        source: None,
    },
    CaptionSpec {
        id: "app.morph_panel.4544f5e90240",
        english: "Exchange the visible shape and the stored shape. Masks and paint stay as they are.",
        source: None,
    },
    CaptionSpec {
        id: "app.morph_panel.57a4942ba374",
        english: "Brushes keep point correspondence. Refine, rebuild or replace the object removes the target; Undo restores it.",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.fef18e1049d0",
        english: "SCULPT LAYERS",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.40616030c681",
        english: "New layer",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.8b4642f04be2",
        english: "Bake visible",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.1f392d24f45e",
        english: "Keep the visible shape and remove all layers, including hidden ones. Undo restores the complete stack.",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.d1e3f59240e2",
        english: "Record shape changes separately, then fade or hide them.",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.60d9ebd387d1",
        english: "Stop recording",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.88d8927cdfc5",
        english: "Editing base shape",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.92bbdec9929d",
        english: "Show or hide this layer. Hiding the recorded layer stops recording.",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.ce56b4b3a72d",
        english: "Intensity",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.2db8e912d1cd",
        english: "Release to apply as one undo step. Fading the recorded layer below 100% stops recording.",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.af0c53fff14b",
        english: "Rename",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.35c9431076a9",
        english: "Record layer",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.59558e3eef38",
        english: "Record on a visible layer at 100% intensity. Paint and masks remain ordinary document edits.",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.6d9383ace6de",
        english: "Delete",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.ef675f65eaaf",
        english: "Remove this layer's shape changes. Undo restores it.",
        source: None,
    },
    CaptionSpec {
        id: "app.layers_panel.1764afb17b44",
        english: "Geometry layers use fixed topology. Bake visible before changing levels or rebuilding. Paint and masks are not layered.",
        source: None,
    },
    CaptionSpec {
        id: "app.mask_region_panel.87fd6bdfef42",
        english: "Through depth on the active part. Alt at press erases. Commands cancel unfinished selections. Radius, pressure and symmetry do not affect regions.",
        source: None,
    },
    CaptionSpec {
        id: "app.mask_filter_panel.9315cd299160",
        english: "Passes",
        source: None,
    },
    CaptionSpec {
        id: "app.mask_filter_panel.af37755afa12",
        english: "Repeat the action 1 to 16 times. More passes reach farther or strengthen the effect. Double-click for keyboard entry.",
        source: None,
    },
    CaptionSpec {
        id: "app.mask_filter_panel.7150e23974a0",
        english: "Amount",
        source: None,
    },
    CaptionSpec {
        id: "app.mask_filter_panel.971bb02cb270",
        english: "Amount per pass. 1 is full effect; 0 keeps the mask unchanged. Double-click for exact keyboard entry; Enter accepts the value.",
        source: None,
    },
    CaptionSpec {
        id: "app.mask_extract_panel.9447b1539bda",
        english: "EXTRACT MASK",
        source: Some(SourceTextKey::SculptExtractTitle),
    },
    CaptionSpec {
        id: "app.mask_extract_panel.4bab4b66a689",
        english: "Turn the dark masked area into a separate surface or shell. Your original part stays in the scene.",
        source: None,
    },
    CaptionSpec {
        id: "app.mask_extract_panel.ccfe760a4456",
        english: "Threshold",
        source: None,
    },
    CaptionSpec {
        id: "app.mask_extract_panel.721f6bef7054",
        english: "Mask weights at or above this value are selected. Double-click for exact keyboard entry; Enter accepts the value.",
        source: None,
    },
    CaptionSpec {
        id: "app.mask_extract_panel.88ad2abfc83b",
        english: "Thickness (world)",
        source: Some(SourceTextKey::SculptExtractThickness),
    },
    CaptionSpec {
        id: "app.mask_extract_panel.792725753b7a",
        english: "Signed thickness in world units: zero makes a surface, positive extends outward, and negative extends inward. Double-click for exact keyboard entry.",
        source: None,
    },
    CaptionSpec {
        id: "app.mask_extract_panel.5c53594d8a50",
        english: "Create part",
        source: Some(SourceTextKey::SculptExtractAction),
    },
    CaptionSpec {
        id: "app.mask_extract_panel.8f9c38536fe0",
        english: "Create a new active SubTool from the mask as one undo step. The source mask stays attached; the new part is unmasked.",
        source: None,
    },
    CaptionSpec {
        id: "app.mask_extract_panel.170f7f889b31",
        english: "Paint a mask first. Lower the threshold to include lighter areas; ambiguous boundaries may need a simpler mask.",
        source: None,
    },
    CaptionSpec {
        id: "app.transform_panel.4e05c6bb6eb2",
        english: "TRANSFORM PART",
        source: None,
    },
    CaptionSpec {
        id: "app.transform_panel.92c0b43b9c6d",
        english: "Transform gizmo · T",
        source: None,
    },
    CaptionSpec {
        id: "app.transform_panel.9a75588a825e",
        english: "Double-click to enter an exact value; Enter accepts the value. Apply changes the part.",
        source: None,
    },
    CaptionSpec {
        id: "app.transform_panel.a903eab2c738",
        english: "Scale",
        source: None,
    },
    CaptionSpec {
        id: "app.transform_panel.79723d0d02e6",
        english: "Positive uniform scale. Double-click for exact keyboard entry.",
        source: None,
    },
    CaptionSpec {
        id: "app.transform_panel.23398e36577d",
        english: "Rotation / scale pivot",
        source: None,
    },
    CaptionSpec {
        id: "app.transform_panel.c3f4196857ec",
        english: "Apply",
        source: None,
    },
    CaptionSpec {
        id: "app.transform_panel.cfb20d20d92c",
        english: "Reset values",
        source: None,
    },
    CaptionSpec {
        id: "app.transform_panel.6c6414320fcc",
        english: "Whole-part transforms ignore mask protection. Paint and mask weights move with their points.",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.337856576635",
        english: "SUBTOOLS",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.85e2a74a55a9",
        english: "Add sphere",
        source: Some(SourceTextKey::SceneAddSphere),
    },
    CaptionSpec {
        id: "app.subtools_panel.a6d087342259",
        english: "Add cube",
        source: Some(SourceTextKey::SceneAddCube),
    },
    CaptionSpec {
        id: "app.subtools_panel.3e08f0d470b5",
        english: "Add cylinder",
        source: Some(SourceTextKey::SceneAddCylinder),
    },
    CaptionSpec {
        id: "app.subtools_panel.4ebafa696187",
        english: "Add torus",
        source: Some(SourceTextKey::SceneAddTorus),
    },
    CaptionSpec {
        id: "app.subtools_panel.8b0c87c10042",
        english: "Append mesh",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.5fb9611fcb10",
        english: "Add OBJ, PLY or STL as a separate part at its saved coordinates. Open loads a whole project.",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.675745b23f75",
        english: "Frame all",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.3adb5023d4ba",
        english: "Choose parts for merge",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.4dc88efdefb6",
        english: "Include this part in the merge",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.805dc198faa1",
        english: "Show or hide this part",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.c92989e6a92f",
        english: "Bake displayed shape for merge",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.49a6e85b293b",
        english: "Consumes retained levels, geometry layers and Morph Targets on chosen parts. Undo restores them.",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.5b8a2f217037",
        english: "Merge chosen parts",
        source: Some(SourceTextKey::SceneMerge),
    },
    CaptionSpec {
        id: "app.subtools_panel.c703ab13b28e",
        english: "Solo active part",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.fb3e700c4706",
        english: "Temporarily hide other parts without changing their visibility flags.",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.2757cdd813c9",
        english: "Clear scene",
        source: Some(SourceTextKey::SceneReset),
    },
    CaptionSpec {
        id: "app.subtools_panel.474bf3df97f6",
        english: "Empty scene. Add a primitive or import a scene to begin.",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.fdd07c4b76a6",
        english: "Duplicate",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.f42efb55a3a5",
        english: "Create a separate copy at the same coordinates, including levels, layers and Morph Target.",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.ce571cad918b",
        english: "Delete",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.9bc4a75bcfff",
        english: "Move up",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.e781304e11f4",
        english: "Move down",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.7bc3eed2f005",
        english: "Rename",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.d64ce52f6362",
        english: "This part is hidden. Show it to sculpt.",
        source: None,
    },
    CaptionSpec {
        id: "app.subtools_panel.5c593338a00c",
        english: "Click a part to select it; Ctrl/Cmd-click its name to select several. Sculpting and OBJ export use the active part. The merge checklist is separate.",
        source: None,
    },
    CaptionSpec {
        id: "app.header.215ed96071ee",
        english: "SCULPTING STUDIO",
        source: None,
    },
    CaptionSpec {
        id: "app.header.ed5bf4431b03",
        english: "Open",
        source: None,
    },
    CaptionSpec {
        id: "app.header.19e565cbc989",
        english: "Open a whole project. Source Ctrl+O/I adds SGL, OBJ, PLY or STL.",
        source: None,
    },
    CaptionSpec {
        id: "app.header.da6df21c4b10",
        english: "Import OBJ or OBrush project · ⌘/Ctrl O",
        source: None,
    },
    CaptionSpec {
        id: "app.header.010baa4fc183",
        english: "Save",
        source: None,
    },
    CaptionSpec {
        id: "app.header.566352be2b27",
        english: "Save project",
        source: None,
    },
    CaptionSpec {
        id: "app.header.a2ddc766055f",
        english: "Save the whole OBrush project using this button. Source S is an unavailable modal picker action.",
        source: None,
    },
    CaptionSpec {
        id: "app.header.57ad8d0f9c5b",
        english: "Save mesh, sculpt layers, morph target, masks, vertex colors and subdivision levels · ⌘/Ctrl S",
        source: None,
    },
    CaptionSpec {
        id: "app.header.2886c2226f0f",
        english: "OBJ",
        source: None,
    },
    CaptionSpec {
        id: "app.header.996e0079e48e",
        english: "Export active OBJ",
        source: None,
    },
    CaptionSpec {
        id: "app.header.ae33d3dd58a6",
        english: "Export the active part as OBJ. Save the OBrush project to retain the whole scene.",
        source: None,
    },
    CaptionSpec {
        id: "app.header.2dd2245cb1a5",
        english: "Object",
        source: None,
    },
    CaptionSpec {
        id: "app.header.5c7e88e89b9b",
        english: "Object, surface, paint, and history controls",
        source: None,
    },
    CaptionSpec {
        id: "app.header.7fc0c24e4a7d",
        english: "Help",
        source: None,
    },
    CaptionSpec {
        id: "app.header.9faf36f3443d",
        english: "Help  H",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.ed1cbe18a173",
        english: "SCULPT",
        source: Some(SourceTextKey::SculptTitle),
    },
    CaptionSpec {
        id: "app.tools_body.0a9341892dd6",
        english: "BRUSH SETTINGS",
        source: Some(SourceTextKey::SculptCommon),
    },
    CaptionSpec {
        id: "app.tools_body.e064c92d289e",
        english: "Radius",
        source: Some(SourceTextKey::SculptRadius),
    },
    CaptionSpec {
        id: "app.tools_body.4cdc2fa5e56b",
        english: "Strength",
        source: Some(SourceTextKey::SculptIntensity),
    },
    CaptionSpec {
        id: "app.tools_body.9b45876ea4dc",
        english: "Small marks need a finer mesh.",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.d3ceefe9068f",
        english: "Use Add subdivision level in Object to add editable detail, or choose Fine for a new sphere. Alpha detail depends on mesh density.",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.678af70390d2",
        english: "Subtract / invert",
        source: Some(SourceTextKey::SculptNegative),
    },
    CaptionSpec {
        id: "app.tools_body.31809cf81efe",
        english: "Hold Alt to temporarily invert the brush",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.97fcf490fddc",
        english: "X symmetry",
        source: Some(SourceTextKey::SculptSymmetry),
    },
    CaptionSpec {
        id: "app.tools_body.a65108620f16",
        english: "Mirror sculpt strokes across the X axis · X",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.417a00c008a1",
        english: "Pressure affects strength",
        source: Some(SourceTextKey::PressureIntensity),
    },
    CaptionSpec {
        id: "app.tools_body.4c39ae12dfa7",
        english: "Pressure affects radius",
        source: Some(SourceTextKey::PressureRadius),
    },
    CaptionSpec {
        id: "app.tools_body.9e1dee7a566a",
        english: "Uses pen pressure when the input device supplies it. Mouse strokes keep full pressure. Move captures its radius when the drag starts.",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.28b3e06d0f8f",
        english: "Morph restores toward the stored shape. Masks protect points; Alt moves away.",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.98589f40ba48",
        english: "None",
        source: Some(SourceTextKey::AlphaNone),
    },
    CaptionSpec {
        id: "app.tools_body.a0ea42396a95",
        english: "Square",
        source: Some(SourceTextKey::AlphaSquare),
    },
    CaptionSpec {
        id: "app.tools_body.a7f63b73c877",
        english: "Skin",
        source: Some(SourceTextKey::AlphaSkin),
    },
    CaptionSpec {
        id: "app.tools_body.d3053e29dd1f",
        english: "Weighted + opacity",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.dc59ba10f835",
        english: "Mean RGB",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.1be6d2f01842",
        english: "Import alpha",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.f4fb2b02b8ff",
        english: "PNG up to 1024×1024. Weighted + opacity uses visible luminance; Mean RGB averages color and ignores opacity. Each brush remembers its selection.",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.ff7bc7f66552",
        english: "Clear",
        source: Some(SourceTextKey::SculptMaskingClear),
    },
    CaptionSpec {
        id: "app.tools_body.56ab4fe19f89",
        english: "Remove from library",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.8b737d16d7b3",
        english: "Rotation",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.ecaa211c172f",
        english: "These Paint settings support locked stamps. Alphas, symmetry and inverse painting remain unavailable.",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.e7f3367aa3c3",
        english: "MASK",
        source: None,
    },
    CaptionSpec {
        id: "app.tools_body.28a8888de6f5",
        english: "Invert",
        source: Some(SourceTextKey::SculptMaskingInvert),
    },
    CaptionSpec {
        id: "app.tools_body.c6a4db3ee261",
        english: "HOLD TO MODIFY",
        source: None,
    },
    CaptionSpec {
        id: "app.brush_options_panel.48ac9c2722b2",
        english: "Size in screen pixels",
        source: None,
    },
    CaptionSpec {
        id: "app.brush_options_panel.ac6f86452736",
        english: "Continuous strokes",
        source: Some(SourceTextKey::SculptContinuous),
    },
    CaptionSpec {
        id: "app.brush_options_panel.cf7b351fa82f",
        english: "Keep applying the brush while held. Release ends the stroke.",
        source: None,
    },
    CaptionSpec {
        id: "app.brush_options_panel.9077d4a91fef",
        english: "Continuous Lock accumulates timer ticks at the press point. Drag sets radius/alpha angle; zero travel holds. Fixed topology and legacy response; UI scale100%. Escape restores the original.",
        source: None,
    },
    CaptionSpec {
        id: "app.brush_options_panel.189b82c8a250",
        english: "Lock stamp center",
        source: Some(SourceTextKey::SculptLockPositon),
    },
    CaptionSpec {
        id: "app.brush_options_panel.c6af1ced3192",
        english: "Drag from the press point to set the radius and rotate the alpha. Escape cancels; release commits one stamp.",
        source: None,
    },
    CaptionSpec {
        id: "app.brush_options_panel.e4d9ec144301",
        english: "Screen radius",
        source: None,
    },
    CaptionSpec {
        id: "app.brush_options_panel.f25e5396ec0e",
        english: "Hardness",
        source: Some(SourceTextKey::SculptHardness),
    },
    CaptionSpec {
        id: "app.brush_options_panel.dca99e6f20a8",
        english: "Cull back-facing points",
        source: Some(SourceTextKey::SculptCulling),
    },
    CaptionSpec {
        id: "app.brush_options_panel.058247451000",
        english: "Mirror X ",
        source: None,
    },
    CaptionSpec {
        id: "app.brush_options_panel.b872669e1c20",
        english: "Clay plane",
        source: Some(SourceTextKey::SculptClay),
    },
    CaptionSpec {
        id: "app.brush_options_panel.4d4a7f97846f",
        english: "Accumulate on current surface",
        source: Some(SourceTextKey::SculptAccumulate),
    },
    CaptionSpec {
        id: "app.brush_options_panel.a0143501678a",
        english: "Tangential smoothing",
        source: Some(SourceTextKey::SculptTangentialSmoothing),
    },
    CaptionSpec {
        id: "app.brush_options_panel.f3975f3fcecb",
        english: "Connected patch only",
        source: Some(SourceTextKey::SculptTopologicalCheck),
    },
    CaptionSpec {
        id: "app.brush_options_panel.ccf95afdbe90",
        english: "Move along surface normal",
        source: Some(SourceTextKey::SculptMoveAlongNormal),
    },
    CaptionSpec {
        id: "app.paint_settings_panel.adf15c2bcbaa",
        english: "Write RGB color",
        source: None,
    },
    CaptionSpec {
        id: "app.paint_settings_panel.a67e1e58e923",
        english: "Write roughness",
        source: None,
    },
    CaptionSpec {
        id: "app.paint_settings_panel.a17bcdb89efd",
        english: "Roughness",
        source: Some(SourceTextKey::SculptRoughness),
    },
    CaptionSpec {
        id: "app.paint_settings_panel.8c3745f29067",
        english: "Write metallic",
        source: None,
    },
    CaptionSpec {
        id: "app.paint_settings_panel.b0c5d68d33cb",
        english: "Metallic",
        source: Some(SourceTextKey::SculptMetallic),
    },
    CaptionSpec {
        id: "app.paint_settings_panel.70f1d6be7e9a",
        english: "Choose PBR in Surface to view roughness and metallic paint.",
        source: None,
    },
    CaptionSpec {
        id: "app.paint_settings_panel.1a8dc4dca79a",
        english: "Enable a channel to paint.",
        source: None,
    },
    CaptionSpec {
        id: "app.paint_settings_panel.729b2cea87bc",
        english: "Fill active part",
        source: None,
    },
    CaptionSpec {
        id: "app.paint_settings_panel.91a20036f554",
        english: "Cancel picker",
        source: None,
    },
    CaptionSpec {
        id: "app.paint_settings_panel.db351844e20b",
        english: "Pick paint",
        source: None,
    },
    CaptionSpec {
        id: "app.paint_settings_panel.b728b3d8af82",
        english: "Click or drag on the active surface to sample paint. Escape cancels.",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector.b3e8d2c9de2a",
        english: "Object & surface",
        source: None,
    },
    CaptionSpec {
        id: "app.source_appearance_panel.45f6a7866079",
        english: "Source object appearance",
        source: None,
    },
    CaptionSpec {
        id: "app.source_appearance_panel.577f3aa38de7",
        english: "PBR",
        source: Some(SourceTextKey::RenderingPbr),
    },
    CaptionSpec {
        id: "app.source_appearance_panel.87b4e2bf9896",
        english: "Flat",
        source: None,
    },
    CaptionSpec {
        id: "app.source_appearance_panel.d72f20ed5ab8",
        english: "Normal",
        source: Some(SourceTextKey::RenderingNormal),
    },
    CaptionSpec {
        id: "app.source_appearance_panel.3e0a4b565275",
        english: "Wireframe",
        source: Some(SourceTextKey::RenderingWireframe),
    },
    CaptionSpec {
        id: "app.source_appearance_panel.0b17f582be32",
        english: "UV",
        source: Some(SourceTextKey::RenderingUv),
    },
    CaptionSpec {
        id: "app.source_appearance_panel.5347433f28de",
        english: "Matcap",
        source: Some(SourceTextKey::RenderingMatcap),
    },
    CaptionSpec {
        id: "app.source_appearance_panel.fd76cc07d179",
        english: "Auxiliary shader",
        source: None,
    },
    CaptionSpec {
        id: "app.source_appearance_panel.3a95f39447fa",
        english: "Source flat shading",
        source: Some(SourceTextKey::RenderingFlat),
    },
    CaptionSpec {
        id: "app.source_appearance_panel.856d2a1f51bf",
        english: "Source wire overlay",
        source: Some(SourceTextKey::RenderingWireframe),
    },
    CaptionSpec {
        id: "app.source_appearance_panel.404bb0a38adc",
        english: "Source transparency",
        source: Some(SourceTextKey::RenderingTransparency),
    },
    CaptionSpec {
        id: "app.source_appearance_panel.7c2d9064fa2a",
        english: "Source appearance is retained per object. The controls below apply to ordinary OBrush objects.",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.c0859e17d49e",
        english: "ACTIVE OBJECT",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.4e87ce4cdfeb",
        english: "Vertices",
        source: Some(SourceTextKey::MeshNbVertices),
    },
    CaptionSpec {
        id: "app.inspector_body.f6c5a19fce27",
        english: "Triangles",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.44d8c149ff26",
        english: "Material points",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.2e9f2849c611",
        english: "UV coordinates",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.9322339dd2b2",
        english: "Original faces and UVs are kept. Automatic detail changes are paused for this surface.",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.3a77138d3be5",
        english: "Remove UVs and original faces",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.5c435ba8362a",
        english: "Standard detail",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.b13460c496e5",
        english: "Fine detail",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.689da08ec6c7",
        english: "High detail",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.b8ea20fb897b",
        english: "Editable detail for the next primitive. Fine supports smaller marks and needs more processing time; face counts differ by shape.",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.7984a45f9f2c",
        english: "Sphere",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.219702b49d4b",
        english: "Cube",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.8edfe3e5a112",
        english: "Choose a scheme before the first retained level. Static modes preserve quad faces and UVs. New static levels carry fine detail with the surface as you edit coarse levels.",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.548ab144f3b7",
        english: "Add subdivision level",
        source: Some(SourceTextKey::MultiresSubdivide),
    },
    CaptionSpec {
        id: "app.inspector_body.5dc265351538",
        english: "Add a finer level and keep the coarse level editable. Select the highest level to add more. Fine detail follows coarse edits; large reshaping can distort it.",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.f05d2176850f",
        english: "Lower",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.e802a8747436",
        english: "Higher",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.3f2a7c23cad7",
        english: "Delete lower",
        source: Some(SourceTextKey::MultiresDelLower),
    },
    CaptionSpec {
        id: "app.inspector_body.7589718a5f7d",
        english: "Delete higher",
        source: Some(SourceTextKey::MultiresDelHigher),
    },
    CaptionSpec {
        id: "app.inspector_body.6b7975da72ca",
        english: "Persistent levels keep fixed topology. Brush detail is limited by the selected level.",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.9d2300ff648f",
        english: "Refine without smoothing",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.3c189345b7e1",
        english: "Add vertices while keeping the active shape exactly. Replaces any subdivision stack; Undo restores it.",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.b81316f6b3a4",
        english: "Bake visible layers to change topology or subdivision levels.",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.f214501f9c03",
        english: "Rebuild / reduce surface",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.7f0d9399cb2c",
        english: "Remeshing can soften thin detail and is not a guaranteed fold repair. Both operations replace subdivision levels and can be undone.",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.75a14b79611f",
        english: "Resolution",
        source: Some(SourceTextKey::RemeshResolution),
    },
    CaptionSpec {
        id: "app.inspector_body.a4277f551bc1",
        english: "Voxel remesh",
        source: Some(SourceTextKey::RemeshRemesh),
    },
    CaptionSpec {
        id: "app.inspector_body.e7c8aeb2b7ab",
        english: "Keep faces",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.8368312596b0",
        english: "Decimate",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.3df38ccf596b",
        english: "Reduce triangles while protecting masks, boundaries and sharp edges. May stop above the requested count to preserve shape.",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.02b587e03098",
        english: "Cancel operation",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.53040dd3efe4",
        english: "SURFACE",
        source: Some(SourceTextKey::RenderingTitle),
    },
    CaptionSpec {
        id: "app.inspector_body.93d818e52343",
        english: "Flat shading",
        source: Some(SourceTextKey::RenderingFlat),
    },
    CaptionSpec {
        id: "app.inspector_body.db684170e645",
        english: "Curvature",
        source: Some(SourceTextKey::RenderingCurvature),
    },
    CaptionSpec {
        id: "app.inspector_body.a09a9445b430",
        english: "Opacity",
        source: Some(SourceTextKey::RenderingTransparency),
    },
    CaptionSpec {
        id: "app.inspector_body.f3f850b891ac",
        english: "Import UV PNG",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.ac4f31bfc3d4",
        english: "Reset to checker",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.f5602b18c2d9",
        english: "PBR exposure",
        source: Some(SourceTextKey::RenderingExposure),
    },
    CaptionSpec {
        id: "app.inspector_body.9accc7445a77",
        english: "Wireframe overlay",
        source: Some(SourceTextKey::RenderingWireframe),
    },
    CaptionSpec {
        id: "app.inspector_body.22b57379aed8",
        english: "W",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.ef491282c4fe",
        english: "Smooth viewport edges",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.f91846123f5b",
        english: "Smooths stair-stepped viewport edges. Geometry resolution still determines sculpt detail.",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.d342b0124270",
        english: "FXAA active",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.48cd47af01ea",
        english: "Basic edges · anti-aliasing unavailable",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.803979d8917f",
        english: "Show mask",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.8309d228ec76",
        english: "Show vertex paint",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.218c4bf11852",
        english: "PAINT COLOR",
        source: Some(SourceTextKey::SculptColor),
    },
    CaptionSpec {
        id: "app.inspector_body.5a56a2f6f982",
        english: "Source Paint color is set in its response controls. Legacy color remains saved.",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.0db451be86c9",
        english: "HISTORY",
        source: Some(SourceTextKey::StateTitle),
    },
    CaptionSpec {
        id: "app.inspector_body.752cc88f4caa",
        english: "Undo",
        source: Some(SourceTextKey::StateUndo),
    },
    CaptionSpec {
        id: "app.inspector_body.707dd6de65d6",
        english: "Redo",
        source: Some(SourceTextKey::StateRedo),
    },
    CaptionSpec {
        id: "app.inspector_body.9ed18c72a105",
        english: "Frame scene   F",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.5c2aeb9985af",
        english: "Viewport",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.09b529e2e0ef",
        english: "Right drag to orbit\nMiddle drag to pan\nScroll to zoom",
        source: None,
    },
    CaptionSpec {
        id: "app.inspector_body.0fcae54e4ce8",
        english: "Rust engine · OpenGL / WebGL2",
        source: None,
    },
    CaptionSpec {
        id: "app.camera_panel.cd829a253d85",
        english: "CAMERA",
        source: Some(SourceTextKey::CameraTitle),
    },
    CaptionSpec {
        id: "app.camera_panel.499eedc4e01e",
        english: "SculptGL static",
        source: None,
    },
    CaptionSpec {
        id: "app.camera_panel.6bfc8c503c21",
        english: "Orthographic",
        source: Some(SourceTextKey::CameraOrthographic),
    },
    CaptionSpec {
        id: "app.camera_panel.5e0b2c13b602",
        english: "FOV ",
        source: Some(SourceTextKey::CameraFov),
    },
    CaptionSpec {
        id: "app.camera_panel.2f4afeb04806",
        english: "View height ",
        source: None,
    },
    CaptionSpec {
        id: "app.camera_panel.c13311ff9ff6",
        english: "Projection zoom ",
        source: None,
    },
    CaptionSpec {
        id: "app.camera_panel.f5d41785d514",
        english: "Static projection; OBrush navigation and Center.",
        source: None,
    },
    CaptionSpec {
        id: "app.camera_panel.b480949ed50e",
        english: "Navigation speed",
        source: None,
    },
    CaptionSpec {
        id: "app.camera_panel.b591fa407121",
        english: "Rotate about picked point",
        source: Some(SourceTextKey::CameraPivot),
    },
    CaptionSpec {
        id: "app.camera_panel.9370b67e7131",
        english: "Picked pivot motion is not yet supported by SculptGL static framing. Choose OBrush framing to use it.",
        source: None,
    },
    CaptionSpec {
        id: "app.camera_panel.565099f6df99",
        english: "Center",
        source: None,
    },
    CaptionSpec {
        id: "app.camera_panel.1f51db0d93a3",
        english: "Press a view again for the opposite side. Hold Shift while rotating to snap.",
        source: None,
    },
    CaptionSpec {
        id: "app.mesh_files_panel.c5875de51dbd",
        english: "MESH FILES",
        source: Some(SourceTextKey::FileTitle),
    },
    CaptionSpec {
        id: "app.mesh_files_panel.b74fe24c7254",
        english: "Scale and center",
        source: Some(SourceTextKey::FileAutoMatrix),
    },
    CaptionSpec {
        id: "app.mesh_files_panel.d0625c68da0d",
        english: "Scale and center only the incoming file's meshes. Existing parts stay in place.",
        source: None,
    },
    CaptionSpec {
        id: "app.mesh_files_panel.c0aab1b128df",
        english: "sRGB vertex color",
        source: Some(SourceTextKey::FileVertexSrgb),
    },
    CaptionSpec {
        id: "app.mesh_files_panel.f3680b8703ea",
        english: "Keep incoming sRGB color values. Uncheck to convert incoming linear RGB to sRGB.",
        source: None,
    },
    CaptionSpec {
        id: "app.mesh_files_panel.37416d9b13ed",
        english: "Add SculptGL file",
        source: None,
    },
    CaptionSpec {
        id: "app.mesh_files_panel.86096f6ba3bc",
        english: "Append SGL, OBJ, PLY or STL. SGL also applies its modern view header. Open replaces a whole project; Reference native captures this browser's typed-array byte order for SGL.",
        source: None,
    },
    CaptionSpec {
        id: "app.mesh_files_panel.1f3e8a33e7ab",
        english: "Mesh exports omit masks, layers and history. Colored STL uses averaged 5-bit face RGB. Save an OBrush project to keep the scene.",
        source: None,
    },
    CaptionSpec {
        id: "app.footer.4b91aa94895e",
        english: "●",
        source: None,
    },
    CaptionSpec {
        id: "app.footer.43e1a5dac71d",
        english: "○",
        source: None,
    },
    CaptionSpec {
        id: "app.footer.9e74cd3d8d6c",
        english: "X SYMMETRY",
        source: None,
    },
    CaptionSpec {
        id: "app.footer.ce3d63dab0cc",
        english: "ASYMMETRIC",
        source: None,
    },
    CaptionSpec {
        id: "app.viewport.08547d0298e8",
        english: "ORTHOGRAPHIC",
        source: None,
    },
    CaptionSpec {
        id: "app.viewport.c7b2af4a18d1",
        english: "PERSPECTIVE",
        source: None,
    },
    CaptionSpec {
        id: "app.viewport.02f933521f03",
        english: "OPEN SCULPTING. NO LIMITS TO YOUR IDEAS.",
        source: None,
    },
    CaptionSpec {
        id: "app.viewport.8a58ca483875",
        english: "TRANSFORM",
        source: None,
    },
    CaptionSpec {
        id: "app.help.b1768462b9d8",
        english: "Welcome to OBrush",
        source: None,
    },
    CaptionSpec {
        id: "app.help.494f5f6a28a4",
        english: "Start with a shape. Make it yours.",
        source: None,
    },
    CaptionSpec {
        id: "app.help.18dcdb07bbf0",
        english: "Drag on the mesh to sculpt. Each stroke is one undo step. Masked vertices are protected; the Paint brush writes per-vertex color.",
        source: None,
    },
    CaptionSpec {
        id: "app.help.9687cb92031a",
        english: "SculptGL basic profile: 0 Move,1 Draw,2 Inflate,3 Twist,4 Smooth,5 Flatten,6 Pinch,7 Crease,8 Drag,9 Paint; E transform, N negative. F/T/L/Space camera and W/R wire/remesh act on release. Ctrl+Z/Y history,Ctrl+O/I file Add,Ctrl+E active OBJ,Ctrl+D active duplicate,I solo,Delete active delete.",
        source: None,
    },
    CaptionSpec {
        id: "app.help.045bef52111d",
        english: "C/X/S modal input, held camera and Source temporary modifier restoration are unavailable. Existing OBrush Shift/Ctrl/Alt gesture behavior remains active. Command keys in text editors are protected.",
        source: None,
    },
    CaptionSpec {
        id: "app.help.515ed995ee97",
        english: "Application preferences",
        source: None,
    },
    CaptionSpec {
        id: "app.help.e3b5addce549",
        english: "Foundation release: separate sculptable parts, persistent subdivision, layers, Morph Targets, remeshing, decimation, pressure controls and PNG alphas. OBJ, PLY, STL and OBrush projects. SculptGL comparisons and full ZBrush parity remain in development.",
        source: None,
    },
    CaptionSpec {
        id: "app.source_paint_panel.cb65cd31c1ac",
        english: "PAINT RESPONSE",
        source: None,
    },
    CaptionSpec {
        id: "app.source_paint_panel.b7cbe4272ad3",
        english: "Legacy",
        source: None,
    },
    CaptionSpec {
        id: "app.source_paint_panel.4bf09ccf0c75",
        english: "Source ordinary response",
        source: None,
    },
    CaptionSpec {
        id: "app.source_paint_panel.8d63821a78eb",
        english: "Inspect complete mask precision",
        source: None,
    },
    CaptionSpec {
        id: "app.source_paint_panel.da6b23e1206a",
        english: "Explicitly allow the reported Float32 roundtrip loss",
        source: None,
    },
    CaptionSpec {
        id: "app.source_paint_panel.c94c5d60b21f",
        english: "Convert the complete active hierarchy",
        source: None,
    },
    CaptionSpec {
        id: "app.source_paint_panel.749a7e33b5db",
        english: "Paint the active part with strokes, continuous painting or locked stamps. Ordinary strokes support imported alphas and object-local X symmetry with a relative mirror offset. Locked stamps require alpha None and symmetry off; inverse and manual rotation are unavailable. Mouse pressure is 0.5. In SculptGL basic shortcuts, hold S after preparation to sample color and material. Fill uses Legacy.",
        source: None,
    },
    CaptionSpec {
        id: "app.source_paint_panel.6595ea93d585",
        english: "Continuous Source Paint",
        source: None,
    },
    CaptionSpec {
        id: "preferences_host.preferences_window.aa1f976ca46f",
        english: "Application preferences",
        source: None,
    },
    CaptionSpec {
        id: "preferences_host.preferences_window.6705c63bd5ae",
        english: "Effective interface: English. Translated captions are wired; selected-language glyph and font admission is pending. OBrush-specific details use English fallback.",
        source: None,
    },
    CaptionSpec {
        id: "preferences_host.preferences_window.8e657d951526",
        english: "Basic non-repeated key phases only. C/X modals, held camera, Shift/Ctrl/Alt restoration and multi-part scene shortcuts are unavailable. Ctrl+O/I selects file Add; Ctrl+E exports active OBJ.",
        source: None,
    },
    CaptionSpec {
        id: "preferences_host.preferences_window.724e01d8184e",
        english: "Custom palette",
        source: None,
    },
    CaptionSpec {
        id: "preferences_host.preferences_window.f6c725c6b0df",
        english: "Overall",
        source: None,
    },
    CaptionSpec {
        id: "preferences_host.preferences_window.b66ebe1dc94c",
        english: "Widget",
        source: None,
    },
    CaptionSpec {
        id: "preferences_host.preferences_window.5f17684a2709",
        english: "Back",
        source: None,
    },
    CaptionSpec {
        id: "preferences_host.preferences_window.9596375fadc7",
        english: "Text",
        source: None,
    },
    CaptionSpec {
        id: "preferences_host.preferences_window.49051aaf77be",
        english: "Borders",
        source: None,
    },
    CaptionSpec {
        id: "preferences_host.preferences_window.28d42bda850a",
        english: "Axes and action accents keep their semantic colors. Mesh materials, background and stored channels are separate.",
        source: None,
    },
    CaptionSpec {
        id: "preferences_host.preferences_window.2367774907f7",
        english: "Cancel model read",
        source: None,
    },
    CaptionSpec {
        id: "uv_bake_host.uv_bake_panel.db546e0641a5",
        english: "Import a mesh with UVs to export its painted textures.",
        source: None,
    },
    CaptionSpec {
        id: "uv_bake_host.uv_bake_panel.f4e1d9942518",
        english: "Baking texture…",
        source: None,
    },
    CaptionSpec {
        id: "uv_bake_host.uv_bake_panel.2d89dd5192e5",
        english: "Cancel texture export",
        source: None,
    },
    CaptionSpec {
        id: "source_geometry_app.source_geometry_panel.3b6794e612e8",
        english: "GEOMETRY RESPONSE",
        source: None,
    },
    CaptionSpec {
        id: "source_geometry_app.source_geometry_panel.28cbd374b931",
        english: "SculptGL (fixed topology)",
        source: None,
    },
    CaptionSpec {
        id: "source_geometry_app.source_geometry_panel.ca96cd519a3e",
        english: "Settings below apply to the SculptGL response. It uses fixed topology; subdivide or remesh to add detail. Radius, intensity and pressure are saved per brush. Alphas and X symmetry are supported. Lock-position stamps and manual alpha rotation are unavailable in this mode.",
        source: None,
    },
    CaptionSpec {
        id: "source_geometry_app.source_geometry_panel.339e0571e3b3",
        english: "Reset transient Source normals",
        source: None,
    },
    CaptionSpec {
        id: "source_geometry_app.source_geometry_panel.1757011fd6fe",
        english: "Clay area plane",
        source: Some(SourceTextKey::SculptClay),
    },
    CaptionSpec {
        id: "source_geometry_app.source_geometry_panel.0961041baab8",
        english: "Accumulate using live distance",
        source: Some(SourceTextKey::SculptAccumulate),
    },
    CaptionSpec {
        id: "source_geometry_app.source_geometry_panel.54ed4988f9f3",
        english: "Continuous ordinary Source",
        source: Some(SourceTextKey::SculptContinuous),
    },
    CaptionSpec {
        id: "source_geometry_app.source_geometry_mask_controls.4dfabf7285ec",
        english: "Inspect complete mask precision",
        source: None,
    },
    CaptionSpec {
        id: "source_geometry_app.source_geometry_mask_controls.1595029c133a",
        english: "Explicitly allow the reported Float32 roundtrip loss",
        source: None,
    },
    CaptionSpec {
        id: "source_geometry_app.source_geometry_mask_controls.826e8e985810",
        english: "Convert the complete active hierarchy",
        source: None,
    },
    CaptionSpec {
        id: "sgl_export_host.mesh_export_action_controls.f0be4e07e1b5",
        english: "Allow omitted mesh attributes",
        source: None,
    },
    CaptionSpec {
        id: "sgl_export_host.mesh_export_action_controls.024fea2a04d9",
        english: "Export SGL scene…",
        source: None,
    },
    CaptionSpec {
        id: "sgl_export_host.mesh_export_action_controls.0c60415d550d",
        english: "OBJ keeps original faces and UVs. These mesh formats omit roughness and metallic; PLY/STL also omit original faces and UVs.",
        source: None,
    },
    CaptionSpec {
        id: "sgl_export_host.mesh_export_action_controls.4bbb63db3489",
        english: "Export active mesh",
        source: None,
    },
    CaptionSpec {
        id: "sgl_export_host.sgl_review_action_controls.c9f29bb8dd49",
        english: "Allow the reported conversions and omissions",
        source: None,
    },
    CaptionSpec {
        id: "sgl_export_host.sgl_review_action_controls.edbf3e66f400",
        english: "Download SGL version 3",
        source: None,
    },
    CaptionSpec {
        id: "sgl_export_host.sgl_review_action_controls.b18728f64dce",
        english: "Cancel",
        source: None,
    },
    CaptionSpec {
        id: "sgl_export_host.sgl_export_review.e6e53cbde7e0",
        english: "SGL scene export review",
        source: None,
    },
    CaptionSpec {
        id: "sgl_export_host.sgl_export_review.d223a8379435",
        english: "Ordinary Studio shading maps to the declared source Matcap constructor; stored vertex paint is kept. This is an appearance conversion.",
        source: None,
    },
    CaptionSpec {
        id: "sgl_export_host.sgl_export_review.9bc0507778fe",
        english: "Symmetry-guide visibility uses the current view preference. Grid and contour flags retain imported values; their rendering remains unavailable. Guide profile/offset and darkening are viewport preferences, outside SGL’s header.",
        source: None,
    },
    CaptionSpec {
        id: "sgl_export_host.sgl_export_review.4ff709b6a931",
        english: "Without an imported display header, grid and contour are explicitly false. Original encoded headers remain unchanged in .obrush provenance.",
        source: None,
    },
    CaptionSpec {
        id: "brush.draw",
        english: "Draw",
        source: Some(SourceTextKey::SculptBrush),
    },
    CaptionSpec {
        id: "brush.clay",
        english: "Clay",
        source: None,
    },
    CaptionSpec {
        id: "brush.inflate",
        english: "Inflate",
        source: Some(SourceTextKey::SculptInflate),
    },
    CaptionSpec {
        id: "brush.smooth",
        english: "Smooth",
        source: Some(SourceTextKey::SculptSmooth),
    },
    CaptionSpec {
        id: "brush.flatten",
        english: "Flatten",
        source: Some(SourceTextKey::SculptFlatten),
    },
    CaptionSpec {
        id: "brush.crease",
        english: "Crease",
        source: Some(SourceTextKey::SculptCrease),
    },
    CaptionSpec {
        id: "brush.move",
        english: "Move",
        source: Some(SourceTextKey::SculptMove),
    },
    CaptionSpec {
        id: "brush.mask",
        english: "Mask",
        source: Some(SourceTextKey::SculptMasking),
    },
    CaptionSpec {
        id: "brush.paint",
        english: "Paint",
        source: Some(SourceTextKey::SculptPaint),
    },
    CaptionSpec {
        id: "brush.morph",
        english: "Morph",
        source: None,
    },
    CaptionSpec {
        id: "brush.pinch",
        english: "Pinch",
        source: Some(SourceTextKey::SculptPinch),
    },
    CaptionSpec {
        id: "brush.twist",
        english: "Twist",
        source: Some(SourceTextKey::SculptTwist),
    },
    CaptionSpec {
        id: "brush.local_scale",
        english: "Local Scale",
        source: Some(SourceTextKey::SculptLocalScale),
    },
    CaptionSpec {
        id: "brush.drag",
        english: "Drag",
        source: Some(SourceTextKey::SculptDrag),
    },
    CaptionSpec {
        id: "camera.orbit",
        english: "Orbit",
        source: Some(SourceTextKey::CameraOrbit),
    },
    CaptionSpec {
        id: "camera.spherical",
        english: "Spherical",
        source: Some(SourceTextKey::CameraSpherical),
    },
    CaptionSpec {
        id: "camera.plane",
        english: "Plane",
        source: Some(SourceTextKey::CameraPlane),
    },
    CaptionSpec {
        id: "shader.studio",
        english: "Studio clay",
        source: None,
    },
    CaptionSpec {
        id: "shader.pbr",
        english: "PBR",
        source: Some(SourceTextKey::RenderingPbr),
    },
    CaptionSpec {
        id: "shader.normal",
        english: "Normal",
        source: Some(SourceTextKey::RenderingNormal),
    },
    CaptionSpec {
        id: "shader.uv",
        english: "UV texture",
        source: Some(SourceTextKey::RenderingUv),
    },
    CaptionSpec {
        id: "shader.matcap",
        english: "Matcap",
        source: Some(SourceTextKey::RenderingMatcap),
    },
    CaptionSpec {
        id: "shader.flat",
        english: "Flat color",
        source: None,
    },
    CaptionSpec {
        id: "paint.radius",
        english: "Radius (viewport points)",
        source: Some(SourceTextKey::SculptRadius),
    },
    CaptionSpec {
        id: "paint.intensity",
        english: "Intensity",
        source: Some(SourceTextKey::SculptIntensity),
    },
    CaptionSpec {
        id: "paint.hardness",
        english: "Hardness",
        source: Some(SourceTextKey::SculptHardness),
    },
    CaptionSpec {
        id: "paint.pressure_intensity",
        english: "Pressure intensity factor",
        source: Some(SourceTextKey::PressureIntensity),
    },
    CaptionSpec {
        id: "paint.pressure_radius",
        english: "Pressure radius factor",
        source: Some(SourceTextKey::PressureRadius),
    },
    CaptionSpec {
        id: "paint.roughness",
        english: "Roughness target",
        source: Some(SourceTextKey::SculptRoughness),
    },
    CaptionSpec {
        id: "paint.metallic",
        english: "Metallic target",
        source: Some(SourceTextKey::SculptMetallic),
    },
    CaptionSpec {
        id: "paint.red",
        english: "Red (0–1)",
        source: None,
    },
    CaptionSpec {
        id: "paint.green",
        english: "Green (0–1)",
        source: None,
    },
    CaptionSpec {
        id: "paint.blue",
        english: "Blue (0–1)",
        source: None,
    },
    CaptionSpec {
        id: "camera.front",
        english: "Front",
        source: Some(SourceTextKey::CameraFront),
    },
    CaptionSpec {
        id: "camera.left",
        english: "Left",
        source: Some(SourceTextKey::CameraLeft),
    },
    CaptionSpec {
        id: "camera.top",
        english: "Top",
        source: Some(SourceTextKey::CameraTop),
    },
    CaptionSpec {
        id: "mask.blur",
        english: "Blur",
        source: Some(SourceTextKey::SculptMaskingBlur),
    },
    CaptionSpec {
        id: "mask.sharpen",
        english: "Sharpen",
        source: Some(SourceTextKey::SculptMaskingSharpen),
    },
    CaptionSpec {
        id: "mask.grow",
        english: "Grow",
        source: None,
    },
    CaptionSpec {
        id: "mask.shrink",
        english: "Shrink",
        source: None,
    },
    CaptionSpec {
        id: "uv.size",
        english: "Texture size",
        source: Some(SourceTextKey::FileExportTextureSize),
    },
    CaptionSpec {
        id: "uv.color",
        english: "Export color PNG",
        source: Some(SourceTextKey::FileExportColor),
    },
    CaptionSpec {
        id: "uv.roughness",
        english: "Export roughness PNG",
        source: Some(SourceTextKey::FileExportRoughness),
    },
    CaptionSpec {
        id: "uv.metallic",
        english: "Export metallic PNG",
        source: Some(SourceTextKey::FileExportMetalness),
    },
    CaptionSpec {
        id: "source_geometry.culling",
        english: "Cull back-facing points",
        source: Some(SourceTextKey::SculptCulling),
    },
    CaptionSpec {
        id: "source_geometry.negative",
        english: "Negative",
        source: Some(SourceTextKey::SculptNegative),
    },
    CaptionSpec {
        id: "matcap.pearl",
        english: "Pearl",
        source: Some(SourceTextKey::MatcapPearl),
    },
    CaptionSpec {
        id: "matcap.clay",
        english: "Clay",
        source: Some(SourceTextKey::MatcapClay),
    },
    CaptionSpec {
        id: "matcap.skin",
        english: "Skin",
        source: Some(SourceTextKey::MatcapSkin),
    },
    CaptionSpec {
        id: "matcap.green",
        english: "Green",
        source: Some(SourceTextKey::MatcapGreen),
    },
    CaptionSpec {
        id: "matcap.white",
        english: "White",
        source: Some(SourceTextKey::MatcapWhite),
    },
    CaptionSpec {
        id: "source_geometry.selection.order",
        english: "Selection order",
        source: None,
    },
    CaptionSpec {
        id: "source_geometry.selection.ordered",
        english: "Ordered static octree",
        source: None,
    },
    CaptionSpec {
        id: "source_geometry.selection.single_leaf",
        english: "Single leaf (at most 100 faces)",
        source: None,
    },
    CaptionSpec {
        id: "viewport.cursor.style",
        english: "Brush cursor",
        source: None,
    },
    CaptionSpec {
        id: "viewport.cursor.studio",
        english: "Studio",
        source: None,
    },
    CaptionSpec {
        id: "viewport.cursor.source_background",
        english: "SculptGL background",
        source: None,
    },
    CaptionSpec {
        id: "viewport.cursor.empty_scope",
        english: "Shows the cursor on a fresh empty view. Use screen-space radius.",
        source: None,
    },
    CaptionSpec {
        id: "source_geometry.prevent_folds",
        english: "Prevent folds",
        source: None,
    },
    CaptionSpec {
        id: "source_geometry.prevent_folds.help",
        english: "Limit deformation to preserve local face quality. Turn off for the full SculptGL brush response, which can fold the surface.",
        source: None,
    },
];
impl CaptionKey {
    pub const ALL: [Self; 365] = [
        Self::CC38AF64CE9B3,
        Self::CADBA4F3F1C64,
        Self::C0903E803CA38,
        Self::CB117A63DA026,
        Self::C10C069826A6D,
        Self::CCC4BF142C03D,
        Self::CF048A0D08F99,
        Self::C630045C3B717,
        Self::C3381DC4E7656,
        Self::C9BD78F90D72D,
        Self::CD282024F3CDC,
        Self::C6E0E4B166896,
        Self::C41CF01C0046A,
        Self::CC7FBCF6BF3A2,
        Self::C0FFE44A69286,
        Self::C5D4BF2363491,
        Self::C7CD6B496C269,
        Self::CDEB97020792A,
        Self::CB52A0B8FA0D0,
        Self::C7B9FFB1D1AD0,
        Self::C412142F1C5A4,
        Self::C463DA7399217,
        Self::C4544F5E90240,
        Self::C57A4942BA374,
        Self::CFEF18E1049D0,
        Self::C40616030C681,
        Self::C8B4642F04BE2,
        Self::C1F392D24F45E,
        Self::CD1E3F59240E2,
        Self::C60D9EBD387D1,
        Self::C88D8927CDFC5,
        Self::C92BBDEC9929D,
        Self::CCE56B4B3A72D,
        Self::C2DB8E912D1CD,
        Self::CAF0C53FFF14B,
        Self::C35C9431076A9,
        Self::C59558E3EEF38,
        Self::C6D9383ACE6DE,
        Self::CEF675F65EAAF,
        Self::C1764AFB17B44,
        Self::C87FD6BDFEF42,
        Self::C9315CD299160,
        Self::CAF37755AFA12,
        Self::C7150E23974A0,
        Self::C971BB02CB270,
        Self::C9447B1539BDA,
        Self::C4BAB4B66A689,
        Self::CCCFE760A4456,
        Self::C721F6BEF7054,
        Self::C88AD2ABFC83B,
        Self::C792725753B7A,
        Self::C5C53594D8A50,
        Self::C8F9C38536FE0,
        Self::C170F7F889B31,
        Self::C4E05C6BB6EB2,
        Self::C92C0B43B9C6D,
        Self::C9A75588A825E,
        Self::CA903EAB2C738,
        Self::C79723D0D02E6,
        Self::C23398E36577D,
        Self::CC3F4196857EC,
        Self::CCFB20D20D92C,
        Self::C6C6414320FCC,
        Self::C337856576635,
        Self::C85E2A74A55A9,
        Self::CA6D087342259,
        Self::C3E08F0D470B5,
        Self::C4EBAFA696187,
        Self::C8B0C87C10042,
        Self::C5FB9611FCB10,
        Self::C675745B23F75,
        Self::C3ADB5023D4BA,
        Self::C4DC88EFDEFB6,
        Self::C805DC198FAA1,
        Self::CC92989E6A92F,
        Self::C49A6E85B293B,
        Self::C5B8A2F217037,
        Self::CC703AB13B28E,
        Self::CFB3E700C4706,
        Self::C2757CDD813C9,
        Self::C474BF3DF97F6,
        Self::CFDD07C4B76A6,
        Self::CF42EFB55A3A5,
        Self::CCE571CAD918B,
        Self::C9BC4A75BCFFF,
        Self::CE781304E11F4,
        Self::C7BC3EED2F005,
        Self::CD64CE52F6362,
        Self::C5C593338A00C,
        Self::C215ED96071EE,
        Self::CED5BF4431B03,
        Self::C19E565CBC989,
        Self::CDA6DF21C4B10,
        Self::C010BAA4FC183,
        Self::C566352BE2B27,
        Self::CA2DDC766055F,
        Self::C57AD8D0F9C5B,
        Self::C2886C2226F0F,
        Self::C996E0079E48E,
        Self::CAE33D3DD58A6,
        Self::C2DD2245CB1A5,
        Self::C5C7E88E89B9B,
        Self::C7FC0C24E4A7D,
        Self::C9FAF36F3443D,
        Self::CED1CBE18A173,
        Self::C0A9341892DD6,
        Self::CE064C92D289E,
        Self::C4CDC2FA5E56B,
        Self::C9B45876EA4DC,
        Self::CD3CEEFE9068F,
        Self::C678AF70390D2,
        Self::C31809CF81EFE,
        Self::C97FCF490FDDC,
        Self::CA65108620F16,
        Self::C417A00C008A1,
        Self::C4C39AE12DFA7,
        Self::C9E1DEE7A566A,
        Self::C28B3E06D0F8F,
        Self::C98589F40BA48,
        Self::CA0EA42396A95,
        Self::CA7F63B73C877,
        Self::CD3053E29DD1F,
        Self::CDC59BA10F835,
        Self::C1BE6D2F01842,
        Self::CF4FB2B02B8FF,
        Self::CFF7BC7F66552,
        Self::C56AB4FE19F89,
        Self::C8B737D16D7B3,
        Self::CECAA211C172F,
        Self::CE7F3367AA3C3,
        Self::C28A8888DE6F5,
        Self::CC6A4DB3EE261,
        Self::C48AC9C2722B2,
        Self::CAC6F86452736,
        Self::CCF7B351FA82F,
        Self::C9077D4A91FEF,
        Self::C189B82C8A250,
        Self::CC6AF1CED3192,
        Self::CE4D9EC144301,
        Self::CF25E5396EC0E,
        Self::CDCA99E6F20A8,
        Self::C058247451000,
        Self::CB872669E1C20,
        Self::C4D4A7F97846F,
        Self::CA0143501678A,
        Self::CF3975F3FCECB,
        Self::CCCF95AFDBE90,
        Self::CADF15C2BCBAA,
        Self::CA67E1E58E923,
        Self::CA17BCDB89EFD,
        Self::C8C3745F29067,
        Self::CB0C5D68D33CB,
        Self::C70F1D6BE7E9A,
        Self::C1A8DC4DCA79A,
        Self::C729B2CEA87BC,
        Self::C91A20036F554,
        Self::CDB351844E20B,
        Self::CB728B3D8AF82,
        Self::CB3E8D2C9DE2A,
        Self::C45F6A7866079,
        Self::C577F3AA38DE7,
        Self::C87B4E2BF9896,
        Self::CD72F20ED5AB8,
        Self::C3E0A4B565275,
        Self::C0B17F582BE32,
        Self::C5347433F28DE,
        Self::CFD76CC07D179,
        Self::C3A95F39447FA,
        Self::C856D2A1F51BF,
        Self::C404BB0A38ADC,
        Self::C7C2D9064FA2A,
        Self::CC0859E17D49E,
        Self::C4E87CE4CDFEB,
        Self::CF6C5A19FCE27,
        Self::C44D8C149FF26,
        Self::C2E9F2849C611,
        Self::C9322339DD2B2,
        Self::C3A77138D3BE5,
        Self::C5C435BA8362A,
        Self::CB13460C496E5,
        Self::C689DA08EC6C7,
        Self::CB8EA20FB897B,
        Self::C7984A45F9F2C,
        Self::C219702B49D4B,
        Self::C8EDFE3E5A112,
        Self::C548AB144F3B7,
        Self::C5DC265351538,
        Self::CF05D2176850F,
        Self::CE802A8747436,
        Self::C3F2A7C23CAD7,
        Self::C7589718A5F7D,
        Self::C6B7975DA72CA,
        Self::C9D2300FF648F,
        Self::C3C189345B7E1,
        Self::CB81316F6B3A4,
        Self::CF214501F9C03,
        Self::C7F0D9399CB2C,
        Self::C75A14B79611F,
        Self::CA4277F551BC1,
        Self::CE7C8AEB2B7AB,
        Self::C8368312596B0,
        Self::C3DF38CCF596B,
        Self::C02B587E03098,
        Self::C53040DD3EFE4,
        Self::C93D818E52343,
        Self::CDB684170E645,
        Self::CA09A9445B430,
        Self::CF3F850B891AC,
        Self::CAC4F31BFC3D4,
        Self::CF5602B18C2D9,
        Self::C9ACCC7445A77,
        Self::C22B57379AED8,
        Self::CEF491282C4FE,
        Self::CF91846123F5B,
        Self::CD342B0124270,
        Self::C48CD47AF01EA,
        Self::C803979D8917F,
        Self::C8309D228EC76,
        Self::C218C4BF11852,
        Self::C5A56A2F6F982,
        Self::C0DB451BE86C9,
        Self::C752CC88F4CAA,
        Self::C707DD6DE65D6,
        Self::C9ED18C72A105,
        Self::C5C2AEB9985AF,
        Self::C09B529E2E0EF,
        Self::C0FCAE54E4CE8,
        Self::CCD829A253D85,
        Self::C499EEDC4E01E,
        Self::C6BFC8C503C21,
        Self::C5E0B2C13B602,
        Self::C2F4AFEB04806,
        Self::CC13311FF9FF6,
        Self::CF5D41785D514,
        Self::CB480949ED50E,
        Self::CB591FA407121,
        Self::C9370B67E7131,
        Self::C565099F6DF99,
        Self::C1F51DB0D93A3,
        Self::CC5875DE51DBD,
        Self::CB74FE24C7254,
        Self::CD0625C68DA0D,
        Self::CC0AAB1B128DF,
        Self::CF3680B8703EA,
        Self::C37416D9B13ED,
        Self::C86096F6BA3BC,
        Self::C1F3E8A33E7AB,
        Self::C4B91AA94895E,
        Self::C43E1A5DAC71D,
        Self::C9E74CD3D8D6C,
        Self::CCE3D63DAB0CC,
        Self::C08547D0298E8,
        Self::CC7B2AF4A18D1,
        Self::C02F933521F03,
        Self::C8A58CA483875,
        Self::CB1768462B9D8,
        Self::C494F5F6A28A4,
        Self::C18DCDB07BBF0,
        Self::C9687CB92031A,
        Self::C045BEF52111D,
        Self::C515ED995EE97,
        Self::CE3B5ADDCE549,
        Self::CCB65CD31C1AC,
        Self::CB7CBE4272AD3,
        Self::C4BF09CCF0C75,
        Self::C8D63821A78EB,
        Self::CDA6B23E1206A,
        Self::CC94C5D60B21F,
        Self::C749A7E33B5DB,
        Self::C6595EA93D585,
        Self::CAA1F976CA46F,
        Self::C6705C63BD5AE,
        Self::C8E657D951526,
        Self::C724E01D8184E,
        Self::CF6C725C6B0DF,
        Self::CB66EBE1DC94C,
        Self::C5F17684A2709,
        Self::C9596375FADC7,
        Self::C49051AAF77BE,
        Self::C28D42BDA850A,
        Self::C2367774907F7,
        Self::CDB546E0641A5,
        Self::CF4E1D9942518,
        Self::C2D89DD5192E5,
        Self::C3B6794E612E8,
        Self::C28CBD374B931,
        Self::CCA96CD519A3E,
        Self::C339E0571E3B3,
        Self::C1757011FD6FE,
        Self::C0961041BAAB8,
        Self::C54ED4988F9F3,
        Self::C4DFABF7285EC,
        Self::C1595029C133A,
        Self::C826E8E985810,
        Self::CF0BE4E07E1B5,
        Self::C024FEA2A04D9,
        Self::C0C60415D550D,
        Self::C4BBB63DB3489,
        Self::CC9F29BB8DD49,
        Self::CEDBF3E66F400,
        Self::CB18728F64DCE,
        Self::CE6E53CBDE7E0,
        Self::CD223A8379435,
        Self::C9BC0507778FE,
        Self::C4FF709B6A931,
        Self::C428977F68F02,
        Self::CE18100AD7A04,
        Self::CF3D2EE694890,
        Self::C388E3C24E35F,
        Self::C5F07E4E57E8E,
        Self::C50EC2A068A9D,
        Self::C052EA75E8A83,
        Self::CDD56571106B5,
        Self::CE291C8967181,
        Self::C732CEFA7F025,
        Self::C995631214BEB,
        Self::C697298D2FC48,
        Self::C36D30A95BF87,
        Self::C6AF7E9EDB552,
        Self::CB0A742941259,
        Self::C343B5567CC00,
        Self::C1D28E6405CCE,
        Self::CADF0F4820146,
        Self::CF86F7FABAA92,
        Self::C1F992AFEED38,
        Self::C019C270B5172,
        Self::CE6BC07E09594,
        Self::CF094D032EAE4,
        Self::CD2E97146E61C,
        Self::CA3BA7BB8E3A4,
        Self::C37B37490F9D1,
        Self::C824E3CA3BB64,
        Self::CB4F9B2FD35C6,
        Self::C855D42092810,
        Self::CF0EB81275BC4,
        Self::C8B35DA5A3251,
        Self::C57F53962541F,
        Self::C61AE79146911,
        Self::C7688CD7B2014,
        Self::CCB106B20524D,
        Self::C360B1EA7EA20,
        Self::C35E9CC1A3A84,
        Self::C08E78D02516B,
        Self::CFF8B0C01664C,
        Self::C5CDC021A9C30,
        Self::CFAA8A951AE15,
        Self::C413F1679B6B3,
        Self::CFAF8AA87390F,
        Self::C85F5FE9A6AF0,
        Self::CB67CE0968F72,
        Self::C2580AF0E9971,
        Self::C29ED1DCAF47D,
        Self::C31D1338A98CF,
        Self::CF42740422670,
        Self::CA660C279F900,
        Self::CC1C8D65B8EDC,
        Self::C2C8B48FE1BC1,
        Self::CB385F53D6E13,
        Self::C34C01497A0F9,
        Self::CursorStyle,
        Self::CursorStudio,
        Self::CursorSourceBackground,
        Self::CursorEmptyScope,
        Self::PreventFolds,
        Self::PreventFoldsHelp,
    ];
    pub const fn id(self) -> &'static str {
        SPECS[self as usize].id
    }
    pub const fn english(self) -> &'static str {
        SPECS[self as usize].english
    }
    pub const fn source(self) -> Option<SourceTextKey> {
        SPECS[self as usize].source
    }
}

/// Conservative data-only gate for callers without a typed font capability.
/// The production shared owner uses install_admitted after a real later pass.
pub const fn effective_locale(_requested: Locale) -> Locale {
    Locale::English
}
pub const fn locale_admitted(requested: Locale) -> bool {
    matches!(requested, Locale::English)
}
#[derive(Clone, Debug)]
pub struct CaptionResolution {
    pub text: String,
    pub requested: Locale,
    pub catalog_used: Locale,
    pub fallback: bool,
    pub source_key: Option<SourceTextKey>,
}
/// Data-only lookup used by admission tests. It cannot enable a UI language.
pub fn resolve_catalog(requested: Locale, key: CaptionKey) -> CaptionResolution {
    let english = || CaptionResolution {
        text: key.english().to_owned(),
        requested,
        catalog_used: Locale::English,
        fallback: requested != Locale::English,
        source_key: key.source(),
    };
    if requested == Locale::English {
        return english();
    }
    let Some(source) = key.source() else {
        return english();
    };
    let Ok(result) = source_text(requested, source, TextArguments::None) else {
        return english();
    };
    let translated = crate::ui_text_fragments::source_fragment(result.catalog_used, source)
        .unwrap_or(result.text.as_ref());
    let text = qualify(key, translated);
    CaptionResolution {
        text,
        requested,
        catalog_used: result.catalog_used,
        fallback: result.fallback,
        source_key: Some(source),
    }
}
fn qualify(key: CaptionKey, text: &str) -> String {
    match key {
        CaptionKey::CD2E97146E61C => format!("{} (viewport points)", text),
        CaptionKey::C824E3CA3BB64 => format!("{} factor", text),
        CaptionKey::CB4F9B2FD35C6 => format!("{} factor", text),
        CaptionKey::C855D42092810 => format!("{} target", text),
        CaptionKey::CF0EB81275BC4 => format!("{} target", text),
        CaptionKey::C88AD2ABFC83B => format!("{} (world)", text),
        CaptionKey::C3A95F39447FA => format!("Source {}", text),
        CaptionKey::C856D2A1F51BF => format!("Source {}", text),
        CaptionKey::C404BB0A38ADC => format!("Source {}", text),
        CaptionKey::CF5602B18C2D9 => format!("PBR {}", text),
        CaptionKey::C413F1679B6B3 => format!("{} PNG", text),
        CaptionKey::CFAF8AA87390F => format!("{} PNG", text),
        CaptionKey::C85F5FE9A6AF0 => format!("{} PNG", text),
        _ => text.to_owned(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TextFrame {
    requested: Locale,
    effective: Locale,
    fonts_ready: bool,
    resolver: ShortcutResolver,
}
impl Default for TextFrame {
    fn default() -> Self {
        Self {
            requested: Locale::English,
            effective: Locale::English,
            fonts_ready: false,
            resolver: ShortcutResolver::new(PreferenceProfile::OBrush),
        }
    }
}
thread_local! {static FRAME:RefCell<TextFrame>=RefCell::new(TextFrame::default());}
/// Conservative frame snapshot for callers without font admission.
/// No document, geometry, file or renderer state.
pub fn install(requested: Locale, resolver: &ShortcutResolver) {
    install_frame(requested, Locale::English, false, resolver);
}
/// Only the shared real font owner can produce the private-field capability.
pub(crate) fn install_admitted(
    capability: crate::ui_fonts::UiFontCapability,
    resolver: &ShortcutResolver,
) {
    install_frame(
        capability.requested(),
        capability.effective(),
        capability.ready(),
        resolver,
    );
}
fn install_frame(
    requested: Locale,
    effective: Locale,
    fonts_ready: bool,
    resolver: &ShortcutResolver,
) {
    FRAME.with(|cell| {
        let mut frame = cell.borrow_mut();
        if frame.requested != requested
            || frame.effective != effective
            || frame.fonts_ready != fonts_ready
            || frame.resolver != *resolver
        {
            frame.requested = requested;
            frame.effective = effective;
            frame.fonts_ready = fonts_ready;
            frame.resolver = resolver.clone();
        }
    });
}

pub fn caption(key: CaptionKey) -> String {
    FRAME.with(|cell| {
        let frame = cell.borrow();
        if frame.resolver.profile() == PreferenceProfile::SculptGl
            && let Some(safe) = source_profile_caption(key)
        {
            return safe.to_owned();
        }
        resolve_catalog(frame.effective, key).text
    })
}
pub fn command_hint(command: Command) -> Option<String> {
    FRAME.with(|cell| cell.borrow().resolver.implemented_command_hint(command))
}
fn source_profile_caption(key: CaptionKey) -> Option<&'static str> {
    match key {
        CaptionKey::C92C0B43B9C6D => Some("Transform gizmo"),
        CaptionKey::C31809CF81EFE => Some(
            "Persistent negative toggle is available in the Source profile. Temporary modifier restoration is unavailable.",
        ),
        CaptionKey::CA65108620F16 => {
            Some("Mirror sculpt strokes across the X axis. Source X radius modal is unavailable.")
        }
        CaptionKey::C9ED18C72A105 => Some("Frame scene"),
        _ => None,
    }
}
pub fn metadata(requested: Locale) -> serde_json::Value {
    FRAME.with(|cell| {
        let frame = cell.borrow();
        let matching = frame.requested == requested;
        let effective = if matching { frame.effective } else { Locale::English };
        let ready = matching && frame.fonts_ready;
        serde_json::json!({"requested":requested.stable_id(),"effective":effective.stable_id(),"glyphs_admitted":ready || requested==Locale::English,"font_assets_installed":ready,"caption_keys":CaptionKey::ALL.len(),"source_equivalent_caption_keys":CaptionKey::ALL.iter().filter(|k|k.source().is_some()).count(),"catalog_data_ready":true,"scope":"Typed shared captions with actual next-pass finite787 font admission. OBrush qualifiers/notices use English fallback; unrestricted user-string Unicode, source font appearance and native/browser layout proof remain separate."})
    })
}

pub fn brush_name(kind: BrushKind) -> String {
    caption(match kind {
        BrushKind::Draw => CaptionKey::C428977F68F02,
        BrushKind::Clay => CaptionKey::CE18100AD7A04,
        BrushKind::Inflate => CaptionKey::CF3D2EE694890,
        BrushKind::Smooth => CaptionKey::C388E3C24E35F,
        BrushKind::Flatten => CaptionKey::C5F07E4E57E8E,
        BrushKind::Crease => CaptionKey::C50EC2A068A9D,
        BrushKind::Move => CaptionKey::C052EA75E8A83,
        BrushKind::Mask => CaptionKey::CDD56571106B5,
        BrushKind::Paint => CaptionKey::CE291C8967181,
        BrushKind::Morph => CaptionKey::C732CEFA7F025,
        BrushKind::Pinch => CaptionKey::C995631214BEB,
        BrushKind::Twist => CaptionKey::C697298D2FC48,
        BrushKind::LocalScale => CaptionKey::C36D30A95BF87,
        BrushKind::Drag => CaptionKey::C6AF7E9EDB552,
    })
}
pub fn camera_mode_name(mode: crate::renderer::CameraMode) -> String {
    caption(match mode {
        crate::renderer::CameraMode::Orbit => CaptionKey::CB0A742941259,
        crate::renderer::CameraMode::Spherical => CaptionKey::C343B5567CC00,
        crate::renderer::CameraMode::Plane => CaptionKey::C1D28E6405CCE,
    })
}
pub fn shading_name(mode: crate::renderer::ShadingMode) -> String {
    caption(match mode {
        crate::renderer::ShadingMode::Studio => CaptionKey::CADF0F4820146,
        crate::renderer::ShadingMode::Pbr => CaptionKey::CF86F7FABAA92,
        crate::renderer::ShadingMode::Normal => CaptionKey::C1F992AFEED38,
        crate::renderer::ShadingMode::Uv => CaptionKey::C019C270B5172,
        crate::renderer::ShadingMode::Matcap => CaptionKey::CE6BC07E09594,
        crate::renderer::ShadingMode::Flat => CaptionKey::CF094D032EAE4,
    })
}
pub fn mask_filter_name(operation: obrush_core::MaskFilterOperation) -> String {
    caption(match operation {
        obrush_core::MaskFilterOperation::Blur => CaptionKey::C35E9CC1A3A84,
        obrush_core::MaskFilterOperation::Sharpen => CaptionKey::C08E78D02516B,
        obrush_core::MaskFilterOperation::Grow => CaptionKey::CFF8B0C01664C,
        obrush_core::MaskFilterOperation::Shrink => CaptionKey::C5CDC021A9C30,
    })
}
pub fn matcap_name(index: usize) -> String {
    match index {
        4 => caption(CaptionKey::C29ED1DCAF47D),
        5 => caption(CaptionKey::C31D1338A98CF),
        6 => caption(CaptionKey::CF42740422670),
        7 => caption(CaptionKey::CA660C279F900),
        8 => caption(CaptionKey::CC1C8D65B8EDC),
        _ => crate::matcap::MATCAP_NAMES
            .get(index)
            .copied()
            .unwrap_or("Invalid Matcap")
            .to_owned(),
    }
}

pub fn source_shader_name(code: u32) -> String {
    caption(match code {
        0 => CaptionKey::C577F3AA38DE7,
        1 => CaptionKey::C87B4E2BF9896,
        2 => CaptionKey::CD72F20ED5AB8,
        3 => CaptionKey::C3E0A4B565275,
        4 => CaptionKey::C0B17F582BE32,
        5 => CaptionKey::C5347433F28DE,
        _ => CaptionKey::CFD76CC07D179,
    })
}
pub fn alpha_selected_name(
    selection: crate::brush_preferences::AlphaSelection,
    imported_name: &str,
) -> String {
    match selection {
        crate::brush_preferences::AlphaSelection::None => caption(CaptionKey::C98589F40BA48),
        crate::brush_preferences::AlphaSelection::Square => caption(CaptionKey::CA0EA42396A95),
        crate::brush_preferences::AlphaSelection::Skin => caption(CaptionKey::CA7F63B73C877),
        crate::brush_preferences::AlphaSelection::Imported(_) => imported_name.to_owned(),
    }
}

#[cfg(test)]
mod caption_admission_tests {
    use super::*;
    use crate::shortcut_profile::{Direction, KeyDelivery, Modal, Modifiers, ShortcutContext};
    struct RestoreFrame(TextFrame);
    impl RestoreFrame {
        fn capture() -> Self {
            Self(FRAME.with(|f| f.borrow().clone()))
        }
    }
    impl Drop for RestoreFrame {
        fn drop(&mut self) {
            FRAME.with(|f| *f.borrow_mut() = self.0.clone());
        }
    }
    #[test]
    fn caption_owner_admission() {
        let _restore = RestoreFrame::capture();
        let ctx = eframe::egui::Context::default();
        let mut owner = crate::ui_fonts::UiFonts::default();
        let resolver = ShortcutResolver::new(PreferenceProfile::OBrush);
        for _ in 0..2 {
            let mut input = eframe::egui::RawInput::default();
            crate::ui_fonts::bound_raw_input(&mut input);
            let _ = ctx.run(input, |ctx| owner.update(ctx, Locale::Japanese));
        }
        install_admitted(owner.capability(), &resolver);
        let spherical = CaptionKey::ALL
            .into_iter()
            .find(|key| key.source() == Some(SourceTextKey::CameraSpherical))
            .unwrap();
        assert_eq!(caption(spherical), "球体 (Trackball)");
        assert_eq!(metadata(Locale::Japanese)["effective"], "japanese");
        assert_eq!(metadata(Locale::Japanese)["font_assets_installed"], true);
        assert_eq!(resolver, ShortcutResolver::new(PreferenceProfile::OBrush));
    }
    #[test]
    fn guarded_install_keeps_english() {
        let _restore = RestoreFrame::capture();
        let resolver = ShortcutResolver::new(PreferenceProfile::OBrush);
        install(Locale::Japanese, &resolver);
        let spherical = CaptionKey::ALL
            .into_iter()
            .find(|key| key.source() == Some(SourceTextKey::CameraSpherical))
            .unwrap();
        assert_eq!(caption(spherical), "Spherical");
        assert_eq!(metadata(Locale::Japanese)["effective"], "english");
        assert_eq!(metadata(Locale::Japanese)["font_assets_installed"], false);
    }
    #[test]
    fn requested_locales_do_not_bypass_selected_glyph_admission() {
        for requested in Locale::ALL {
            assert_eq!(effective_locale(requested), Locale::English);
            assert_eq!(locale_admitted(requested), requested == Locale::English);
            assert_eq!(metadata(requested)["font_assets_installed"], false);
        }
    }
    #[test]
    fn public_caption_ids_are_unique_bounded_and_never_commands() {
        let mut ids = CaptionKey::ALL.map(CaptionKey::id);
        ids.sort();
        assert!(ids.windows(2).all(|w| w[0] != w[1]));
        assert!(ids.iter().all(|id| id.len() < 160));
        assert!(
            CaptionKey::ALL
                .iter()
                .all(|key| key.english().len() <= 4096)
        );
    }
    #[test]
    fn shader_display_words_keep_actual_legacy_renderer_literals() {
        use crate::renderer::ShadingMode::*;
        let _restore = RestoreFrame::capture();
        install(
            Locale::English,
            &ShortcutResolver::new(PreferenceProfile::OBrush),
        );
        for (mode, expected) in [
            (Studio, "Studio clay"),
            (Pbr, "PBR"),
            (Normal, "Normal"),
            (Uv, "UV texture"),
            (Matcap, "Matcap"),
            (Flat, "Flat color"),
        ] {
            assert_eq!(shading_name(mode), expected);
        }
    }
    #[test]
    fn brush_display_words_keep_original_tile_roster_and_morph_scope() {
        let _restore = RestoreFrame::capture();
        install(
            Locale::English,
            &ShortcutResolver::new(PreferenceProfile::OBrush),
        );
        for (kind, expected) in [
            (BrushKind::Draw, "Draw"),
            (BrushKind::Clay, "Clay"),
            (BrushKind::Inflate, "Inflate"),
            (BrushKind::Smooth, "Smooth"),
            (BrushKind::Flatten, "Flatten"),
            (BrushKind::Crease, "Crease"),
            (BrushKind::Move, "Move"),
            (BrushKind::Mask, "Mask"),
            (BrushKind::Paint, "Paint"),
            (BrushKind::Morph, "Morph"),
            (BrushKind::Pinch, "Pinch"),
            (BrushKind::Twist, "Twist"),
            (BrushKind::LocalScale, "Local Scale"),
            (BrushKind::Drag, "Drag"),
        ] {
            assert_eq!(brush_name(kind), expected);
        }
    }
    #[test]
    fn english_legacy_parameter_words_and_units_are_literal() {
        assert_eq!(
            resolve_catalog(Locale::English, CaptionKey::CD2E97146E61C).text,
            "Radius (viewport points)"
        );
        assert_eq!(
            resolve_catalog(Locale::English, CaptionKey::CB4F9B2FD35C6).text,
            "Pressure radius factor"
        );
        assert_eq!(
            resolve_catalog(Locale::English, CaptionKey::C855D42092810).text,
            "Roughness target"
        );
        assert_eq!(
            resolve_catalog(Locale::English, CaptionKey::CE064C92D289E).text,
            "Radius"
        );
        assert_eq!(
            resolve_catalog(Locale::English, CaptionKey::C4CDC2FA5E56B).text,
            "Strength"
        );
    }
    #[test]
    fn translated_radius_cannot_advertise_unimplemented_x_modal() {
        let raw = source_text(
            Locale::German,
            SourceTextKey::SculptRadius,
            TextArguments::None,
        )
        .unwrap();
        let translated = resolve_catalog(Locale::German, CaptionKey::CD2E97146E61C);
        assert_eq!(
            translated.text,
            format!(
                "{} (viewport points)",
                crate::ui_text_fragments::source_fragment(
                    raw.catalog_used,
                    SourceTextKey::SculptRadius
                )
                .unwrap()
            )
        );
        assert!(!translated.text.contains("(-X)"));
    }
    #[test]
    fn descriptive_orbit_parentheses_are_retained_exactly() {
        for locale in Locale::ALL {
            if locale != Locale::English {
                let raw =
                    source_text(locale, SourceTextKey::CameraOrbit, TextArguments::None).unwrap();
                assert_eq!(
                    resolve_catalog(locale, CaptionKey::CB0A742941259).text,
                    raw.text
                );
            }
        }
    }
    #[test]
    fn owned_morph_and_standalone_clay_do_not_infer_reference_features() {
        for locale in Locale::ALL {
            for (key, expected) in [
                (CaptionKey::C732CEFA7F025, "Morph"),
                (CaptionKey::CE18100AD7A04, "Clay"),
            ] {
                let text = resolve_catalog(locale, key);
                assert_eq!(text.text, expected);
                assert_eq!(text.source_key, None);
            }
        }
    }
    #[test]
    fn all_catalog_caption_lookups_preserve_bounded_plain_text_and_fallback() {
        for locale in Locale::ALL {
            for key in CaptionKey::ALL {
                let result = resolve_catalog(locale, key);
                assert!(!result.text.is_empty());
                assert!(result.text.len() <= 8192);
                assert_eq!(result.requested, locale);
            }
        }
    }
    #[test]
    fn repeated_install_and_lookup_keep_resolver_bits_and_guarded_english() {
        let _restore = RestoreFrame::capture();
        let resolver = ShortcutResolver::new(PreferenceProfile::SculptGl);
        let before = resolver.clone();
        install(Locale::Japanese, &resolver);
        for _ in 0..16 {
            assert_eq!(brush_name(BrushKind::Draw), "Draw");
            install(Locale::Japanese, &resolver);
        }
        assert_eq!(resolver, before);
        assert_eq!(metadata(Locale::Japanese)["effective"], "english");
    }
    #[test]
    fn legacy_f_frames_but_source_f_front_uses_actual_release() {
        let legacy = ShortcutResolver::new(PreferenceProfile::OBrush);
        let source = ShortcutResolver::new(PreferenceProfile::SculptGl);
        let f = legacy.implemented_hint_event(Command::FrameScene).unwrap();
        assert_eq!(f.code, 70);
        assert!(f.pressed);
        assert!(legacy.implemented_hint_event(Command::ViewFront).is_none());
        let f = source.implemented_hint_event(Command::ViewFront).unwrap();
        assert_eq!(f.code, 70);
        assert!(!f.pressed);
    }
    #[test]
    fn hints_exclude_unimplemented_modals_and_contextual_snap() {
        for profile in [PreferenceProfile::OBrush, PreferenceProfile::SculptGl] {
            let r = ShortcutResolver::new(profile);
            for m in [Modal::Radius, Modal::Intensity, Modal::Picker] {
                assert!(r.implemented_hint_event(Command::ModalStart(m)).is_none());
                assert!(r.implemented_hint_event(Command::ModalEnd(m)).is_none());
            }
            assert!(r.implemented_hint_event(Command::SnapView).is_none());
        }
        let source = ShortcutResolver::new(PreferenceProfile::SculptGl);
        let left = source
            .implemented_hint_event(Command::Strafe(Direction::Left, true))
            .unwrap();
        assert_eq!(left.code, 37);
        assert!(left.pressed);
        assert!(
            ShortcutResolver::new(PreferenceProfile::OBrush)
                .implemented_hint_event(Command::Strafe(Direction::Left, true))
                .is_none()
        );
    }
    #[test]
    fn configured_source_front_alias_is_observed_not_guessed_from_catalog_f() {
        let mut r = ShortcutResolver::new(PreferenceProfile::SculptGl);
        r.source_overrides("camera_front:Q,paint:F").unwrap();
        let e = r.implemented_hint_event(Command::ViewFront).unwrap();
        assert_eq!(e.code, 81);
        assert!(!e.pressed);
        assert!(
            r.resolve(e, ShortcutContext::default())
                .contains(&Command::ViewFront)
        );
        assert!(
            !r.resolve(KeyDelivery { code: 70, ..e }, ShortcutContext::default())
                .contains(&Command::ViewFront)
        );
    }
    #[test]
    fn source_front_added_alias_retains_default_f_and_new_q() {
        let mut r = ShortcutResolver::new(PreferenceProfile::SculptGl);
        r.source_overrides("camera_front:Q").unwrap();
        let hint = r.implemented_hint_event(Command::ViewFront).unwrap();
        assert_eq!(hint.code, 70);
        assert!(!hint.pressed);
        for code in [70, 81] {
            assert_eq!(
                r.resolve(
                    KeyDelivery {
                        code,
                        pressed: false,
                        repeat: false,
                        modifiers: Modifiers::default(),
                    },
                    ShortcutContext::default()
                ),
                vec![Command::ViewFront]
            );
        }
    }
    #[test]
    fn every_emitted_hint_delivers_exact_same_implemented_owner() {
        for profile in [PreferenceProfile::OBrush, PreferenceProfile::SculptGl] {
            let r = ShortcutResolver::new(profile);
            for command in [
                Command::Wireframe,
                Command::Remesh,
                Command::ToggleNegative,
                Command::ViewFront,
                Command::ViewTop,
                Command::ViewLeft,
                Command::FrameScene,
                Command::SelectTransform,
                Command::ToggleTransform,
                Command::Undo,
                Command::Redo,
                Command::SaveProject,
                Command::OpenProject,
                Command::AddFile,
                Command::ExportObj,
                Command::ClearScene,
                Command::Duplicate,
                Command::DeleteSelection,
            ] {
                if let Some(event) = r.implemented_hint_event(command) {
                    assert!(
                        r.resolve(event, ShortcutContext::default())
                            .contains(&command)
                    );
                    assert!(r.implemented_command_hint(command).is_some());
                }
            }
        }
    }
    #[test]
    fn source_profile_hides_legacy_f_inline_without_enabling_modals() {
        let _restore = RestoreFrame::capture();
        let legacy = ShortcutResolver::new(PreferenceProfile::OBrush);
        install(Locale::English, &legacy);
        assert_eq!(caption(CaptionKey::C9ED18C72A105), "Frame scene   F");
        install(
            Locale::English,
            &ShortcutResolver::new(PreferenceProfile::SculptGl),
        );
        assert_eq!(caption(CaptionKey::C9ED18C72A105), "Frame scene");
        assert!(command_hint(Command::ModalStart(Modal::Radius)).is_none());
    }
    #[test]
    fn editor_owned_release_never_becomes_a_hint_command_execution() {
        let r = ShortcutResolver::new(PreferenceProfile::SculptGl);
        let e = KeyDelivery {
            code: 70,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::default(),
        };
        assert_eq!(
            r.resolve(
                e,
                ShortcutContext {
                    gui_focus: true,
                    ..Default::default()
                }
            ),
            Vec::<Command>::new()
        );
    }
    #[test]
    fn source_front_keyup_repeat_flag_does_not_change_release_semantics() {
        let r = ShortcutResolver::new(PreferenceProfile::SculptGl);
        // GuiCamera.onKeyUp checks GUI ownership and key mapping, not repeat.
        for repeat in [false, true] {
            assert_eq!(
                r.resolve(
                    KeyDelivery {
                        code: 70,
                        pressed: false,
                        repeat,
                        modifiers: Modifiers::default(),
                    },
                    ShortcutContext::default()
                ),
                vec![Command::ViewFront]
            );
        }
    }
}
