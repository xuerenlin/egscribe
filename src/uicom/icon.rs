#[allow(non_camel_case_types)]
#[derive(Clone)]
pub enum IconName {
    icon_chevron_left,      //e900
    icon_chevron_down,      //e901
    icon_format_font_size,  //e902
    icon_wrap_text,         //e903
    icon_sort_numerically,  //e904
    icon_external_link,     //e905
    icon_home, //e906
    icon_refresh, //e907
    icon_file_rename, //e908
    icon_delete, //e909
    icon_new, //e90A
    icon_unfixed, //e90B
    icon_fixed, //e90C
    icon_close, //e90d
    icon_indent_decrease,  //e90e
    icon_indent_increase,  //e90f
    icon_close1,   //e910
    icon_cross, //e911
    icon_clear, //e912
    icon_document_text, //e913
    icon_user_x, //e914
    icon_file_minus, //e915
    icon_user_check, //e916
    icon_file_plus, //e917
    icon_file_text, //e918
    icon_users, //e919
    icon_user_minus, //e91a
    icon_user_plus, //e91b
    icon_user1, //e91c
    icon_file1, //e91d
    icon_file_gif, //e91e
    icon_file_gif1, //e91f
    icon_file_rar, //e920
    icon_file_rar1, //e921
    icon_file_iso, //e922
    icon_file_iso1, //e923
    icon_file_dmg, //e924
    icon_file_dmg1, //e925
    icon_file_tgz, //e926
    icon_file_tgz1, //e927
    icon_file_ott, //e928
    icon_file_ott1, //e929
    icon_file_wav, //e92a
    icon_file_wav1, //e92b
    icon_file_aac, //e92c
    icon_file_aac1, //e92d
    icon_file_aiff, //e92e
    icon_file_aiff1, //e92f
    icon_file_3gp, //e930
    icon_file_3gp1, //e931
    icon_file_mid, //e932
    icon_file_mid1, //e933
    icon_file_quicktime, //e934
    icon_file_quicktime1, //e935
    icon_file_mpg, //e936
    icon_file_mpg1, //e937
    icon_file_flv, //e938
    icon_file_flv1, //e939
    icon_file_m4v, //e93a
    icon_file_m4v1, //e93b
    icon_file_rtf, //e93c
    icon_file_rtf1, //e93d
    icon_file_txt, //e93e
    icon_file_txt1, //e93f
    icon_file_dot, //e940
    icon_file_dot1, //e941
    icon_file_pps, //e942
    icon_file_pps1, //e943
    icon_file_ods, //e944
    icon_file_ods1, //e945
    icon_file_xlsx, //e946
    icon_file_xlsx1, //e947
    icon_file_dotx, //e948
    icon_file_dotx1, //e949
    icon_file_odp, //e94a
    icon_file_odp1, //e94b
    icon_file_avi, //e94c
    icon_file_avi1, //e94d
    icon_file_exe, //e94e
    icon_file_exe1, //e94f
    icon_file_h, //e950
    icon_file_h1, //e951
    icon_file_yml, //e952
    icon_file_yml1, //e953
    icon_file_dat, //e954
    icon_file_dat1, //e955
    icon_file_ics, //e956
    icon_file_ics1, //e957
    icon_file_asp, //e958
    icon_file_asp1, //e959
    icon_file_ppt, //e95a
    icon_file_ppt1, //e95b
    icon_file_docx, //e95c
    icon_file_docx1, //e95d
    icon_file_xls, //e95e
    icon_file_xls1, //e95f
    icon_file_odt, //e960
    icon_file_odt1, //e961
    icon_file_doc, //e962
    icon_file_doc1, //e963
    icon_file_dxf, //e964
    icon_file_dxf1, //e965
    icon_file_tga, //e966
    icon_file_tga1, //e967
    icon_file_cpp, //e968
    icon_file_cpp1, //e969
    icon_file_rb, //e96a
    icon_file_rb1, //e96b
    icon_file_sql, //e96c
    icon_file_sql1, //e96d
    icon_file_c, //e96e
    icon_file_c1, //e96f
    icon_file_py, //e970
    icon_file_py1, //e971
    icon_file_php, //e972
    icon_file_php1, //e973
    icon_file_ots, //e974
    icon_file_ots1, //e975
    icon_file_tiff, //e976
    icon_file_tiff1, //e977
    icon_file_eps, //e978
    icon_file_eps1, //e979
    icon_file_dwg, //e97a
    icon_file_dwg1, //e97b
    icon_file_bmp, //e97c
    icon_file_bmp1, //e97d
    icon_file_ai, //e97e
    icon_file_ai1, //e97f
    icon_file_psd, //e980
    icon_file_psd1, //e981
    icon_file_java, //e982
    icon_file_java1, //e983
    icon_file_html, //e984
    icon_file_html1, //e985
    icon_file_key, //e986
    icon_file_key1, //e987
    icon_file_jpg, //e988
    icon_file_jpg1, //e989
    icon_file_mov, //e98a
    icon_file_mov1, //e98b
    icon_file_mp4, //e98c
    icon_file_mp41, //e98d
    icon_file_mp3, //e98e
    icon_file_mp31, //e98f
    icon_file_png, //e990
    icon_file_png1, //e991
    icon_file_app, //e992
    icon_file_app1, //e993
    icon_file_pdf, //e994
    icon_file_pdf1, //e995
    icon_file_css, //e996
    icon_file_css1, //e997
    icon_file_xml, //e998
    icon_file_xml1, //e999
    icon_file_zip, //e99a
    icon_file_zip1, //e99b
    icon_text_document, //e99c
    icon_text_document_inverted, //e99d
    icon_add_user, //e99e
    icon_remove_user, //e99f
    icon_document_landscape, //e9a0
    icon_documents, //e9a1
    icon_users1, //e9a2
    icon_user, //e9a3
    icon_document1, //e9a4
    icon_bookmark_outline_add, //e9a5
    icon_bookmark_outline, //e9a6
    icon_bookmark, //e9a7
    icon_bookmark1, //e9a8
    icon_bookmark2, //e9a9
    icon_bookmark3, //e9aa
    icon_sunset, //e9ab
    icon_sunrise, //e9ac
    icon_sunrise1, //e9ad
    icon_sun, //e9ae
    icon_tag_cord, //e9af
    icon_tag_cord1, //e9b0
    icon_puzzle, //e9b1
    icon_mic, //e9b2
    icon_hash, //e9c7
}

impl IconName {
    pub fn to_char(&self) -> char {
        match self {
            IconName::icon_chevron_left  => '\u{e900}',
            IconName::icon_chevron_down => '\u{e901}',
            IconName::icon_format_font_size => '\u{e902}',
            IconName::icon_wrap_text => '\u{e903}', 
            IconName::icon_sort_numerically => '\u{e904}',
            IconName::icon_external_link => '\u{e905}', 
            IconName::icon_home => '\u{e906}', 
            IconName::icon_refresh => '\u{e907}',
            IconName::icon_file_rename => '\u{e908}', 
            IconName::icon_delete => '\u{e909}',
            IconName::icon_new => '\u{e90A}',
            IconName::icon_unfixed => '\u{e90B}',
            IconName::icon_fixed => '\u{e90C}',
            IconName::icon_close => '\u{e90D}',
            IconName::icon_indent_decrease => '\u{e90E}',
            IconName::icon_indent_increase => '\u{e90F}',
            IconName::icon_close1 => '\u{e910}',
            IconName::icon_cross => '\u{e911}',
            IconName::icon_clear => '\u{e912}',
            IconName::icon_document_text => '\u{e913}',
            IconName::icon_user_x => '\u{e914}',
            IconName::icon_file_minus => '\u{e915}',
            IconName::icon_user_check => '\u{e916}',
            IconName::icon_file_plus => '\u{e917}',
            IconName::icon_file_text => '\u{e918}',
            IconName::icon_users => '\u{e919}',
            IconName::icon_user_minus => '\u{e91a}',
            IconName::icon_user_plus => '\u{e91b}',
            IconName::icon_user1 => '\u{e91c}',
            IconName::icon_file1 => '\u{e91d}',
            IconName::icon_file_gif => '\u{e91e}',
            IconName::icon_file_gif1 => '\u{e91f}',
            IconName::icon_file_rar => '\u{e920}',
            IconName::icon_file_rar1 => '\u{e921}',
            IconName::icon_file_iso => '\u{e922}',
            IconName::icon_file_iso1 => '\u{e923}',
            IconName::icon_file_dmg => '\u{e924}',
            IconName::icon_file_dmg1 => '\u{e925}',
            IconName::icon_file_tgz => '\u{e926}',
            IconName::icon_file_tgz1 => '\u{e927}',
            IconName::icon_file_ott => '\u{e928}',
            IconName::icon_file_ott1 => '\u{e929}',
            IconName::icon_file_wav => '\u{e92a}',
            IconName::icon_file_wav1 => '\u{e92b}',
            IconName::icon_file_aac => '\u{e92c}',
            IconName::icon_file_aac1 => '\u{e92d}',
            IconName::icon_file_aiff => '\u{e92e}',
            IconName::icon_file_aiff1 => '\u{e92f}',
            IconName::icon_file_3gp => '\u{e930}',
            IconName::icon_file_3gp1 => '\u{e931}',
            IconName::icon_file_mid => '\u{e932}',
            IconName::icon_file_mid1 => '\u{e933}',
            IconName::icon_file_quicktime => '\u{e934}',
            IconName::icon_file_quicktime1 => '\u{e935}',
            IconName::icon_file_mpg => '\u{e936}',
            IconName::icon_file_mpg1 => '\u{e937}',
            IconName::icon_file_flv => '\u{e938}',
            IconName::icon_file_flv1 => '\u{e939}',
            IconName::icon_file_m4v => '\u{e93a}',
            IconName::icon_file_m4v1 => '\u{e93b}',
            IconName::icon_file_rtf => '\u{e93c}',
            IconName::icon_file_rtf1 => '\u{e93d}',
            IconName::icon_file_txt => '\u{e93e}',
            IconName::icon_file_txt1 => '\u{e93f}',
            IconName::icon_file_dot => '\u{e940}',
            IconName::icon_file_dot1 => '\u{e941}',
            IconName::icon_file_pps => '\u{e942}',
            IconName::icon_file_pps1 => '\u{e943}',
            IconName::icon_file_ods => '\u{e944}',
            IconName::icon_file_ods1 => '\u{e945}',
            IconName::icon_file_xlsx => '\u{e946}',
            IconName::icon_file_xlsx1 => '\u{e947}',
            IconName::icon_file_dotx => '\u{e948}',
            IconName::icon_file_dotx1 => '\u{e949}',
            IconName::icon_file_odp => '\u{e94a}',
            IconName::icon_file_odp1 => '\u{e94b}',
            IconName::icon_file_avi => '\u{e94c}',
            IconName::icon_file_avi1 => '\u{e94d}',
            IconName::icon_file_exe => '\u{e94e}',
            IconName::icon_file_exe1 => '\u{e94f}',
            IconName::icon_file_h => '\u{e950}',
            IconName::icon_file_h1 => '\u{e951}',
            IconName::icon_file_yml => '\u{e952}',
            IconName::icon_file_yml1 => '\u{e953}',
            IconName::icon_file_dat => '\u{e954}',
            IconName::icon_file_dat1 => '\u{e955}',
            IconName::icon_file_ics => '\u{e956}',
            IconName::icon_file_ics1 => '\u{e957}',
            IconName::icon_file_asp => '\u{e958}',
            IconName::icon_file_asp1 => '\u{e959}',
            IconName::icon_file_ppt => '\u{e95a}',
            IconName::icon_file_ppt1 => '\u{e95b}',
            IconName::icon_file_docx => '\u{e95c}',
            IconName::icon_file_docx1 => '\u{e95d}',
            IconName::icon_file_xls => '\u{e95e}',
            IconName::icon_file_xls1 => '\u{e95f}',
            IconName::icon_file_odt => '\u{e960}',
            IconName::icon_file_odt1 => '\u{e961}',
            IconName::icon_file_doc => '\u{e962}',
            IconName::icon_file_doc1 => '\u{e963}',
            IconName::icon_file_dxf => '\u{e964}',
            IconName::icon_file_dxf1 => '\u{e965}',
            IconName::icon_file_tga => '\u{e966}',
            IconName::icon_file_tga1 => '\u{e967}',
            IconName::icon_file_cpp => '\u{e968}',
            IconName::icon_file_cpp1 => '\u{e969}',
            IconName::icon_file_rb => '\u{e96a}',
            IconName::icon_file_rb1 => '\u{e96b}',
            IconName::icon_file_sql => '\u{e96c}',
            IconName::icon_file_sql1 => '\u{e96d}',
            IconName::icon_file_c => '\u{e96e}',
            IconName::icon_file_c1 => '\u{e96f}',
            IconName::icon_file_py => '\u{e970}',
            IconName::icon_file_py1 => '\u{e971}',
            IconName::icon_file_php => '\u{e972}',
            IconName::icon_file_php1 => '\u{e973}',
            IconName::icon_file_ots => '\u{e974}',
            IconName::icon_file_ots1 => '\u{e975}',
            IconName::icon_file_tiff => '\u{e976}',
            IconName::icon_file_tiff1 => '\u{e977}',
            IconName::icon_file_eps => '\u{e978}',
            IconName::icon_file_eps1 => '\u{e979}',
            IconName::icon_file_dwg => '\u{e97a}',
            IconName::icon_file_dwg1 => '\u{e97b}',
            IconName::icon_file_bmp => '\u{e97c}',
            IconName::icon_file_bmp1 => '\u{e97d}',
            IconName::icon_file_ai => '\u{e97e}',
            IconName::icon_file_ai1 => '\u{e97f}',
            IconName::icon_file_psd => '\u{e980}',
            IconName::icon_file_psd1 => '\u{e981}',
            IconName::icon_file_java => '\u{e982}',
            IconName::icon_file_java1 => '\u{e983}',
            IconName::icon_file_html => '\u{e984}',
            IconName::icon_file_html1 => '\u{e985}',
            IconName::icon_file_key => '\u{e986}',
            IconName::icon_file_key1 => '\u{e987}',
            IconName::icon_file_jpg => '\u{e988}',
            IconName::icon_file_jpg1 => '\u{e989}',
            IconName::icon_file_mov => '\u{e98a}',
            IconName::icon_file_mov1 => '\u{e98b}',
            IconName::icon_file_mp4 => '\u{e98c}',
            IconName::icon_file_mp41 => '\u{e98d}',
            IconName::icon_file_mp3 => '\u{e98e}',
            IconName::icon_file_mp31 => '\u{e98f}',
            IconName::icon_file_png => '\u{e990}',
            IconName::icon_file_png1 => '\u{e991}',
            IconName::icon_file_app => '\u{e992}',
            IconName::icon_file_app1 => '\u{e993}',
            IconName::icon_file_pdf => '\u{e994}',
            IconName::icon_file_pdf1 => '\u{e995}',
            IconName::icon_file_css => '\u{e996}',
            IconName::icon_file_css1 => '\u{e997}',
            IconName::icon_file_xml => '\u{e998}',
            IconName::icon_file_xml1 => '\u{e999}',
            IconName::icon_file_zip => '\u{e99a}',
            IconName::icon_file_zip1 => '\u{e99b}',
            IconName::icon_text_document => '\u{e99c}',
            IconName::icon_text_document_inverted => '\u{e99d}',
            IconName::icon_add_user => '\u{e99e}',
            IconName::icon_remove_user => '\u{e99f}',
            IconName::icon_document_landscape => '\u{e9a0}',
            IconName::icon_documents => '\u{e9a1}',
            IconName::icon_users1 => '\u{e9a2}',
            IconName::icon_user => '\u{e9a3}',
            IconName::icon_document1 => '\u{e9a4}',
            IconName::icon_bookmark_outline_add => '\u{e9a5}',
            IconName::icon_bookmark_outline => '\u{e9a6}',
            IconName::icon_bookmark => '\u{e9a7}',
            IconName::icon_bookmark1 => '\u{e9a8}',
            IconName::icon_bookmark2 => '\u{e9a9}',
            IconName::icon_bookmark3 => '\u{e9aa}',
            IconName::icon_sunset => '\u{e9ab}',
            IconName::icon_sunrise => '\u{e9ac}',
            IconName::icon_sunrise1 => '\u{e9ad}',
            IconName::icon_sun => '\u{e9ae}',
            IconName::icon_tag_cord => '\u{e9af}',
            IconName::icon_tag_cord1 => '\u{e9b0}',
            IconName::icon_puzzle => '\u{e9b1}',
            IconName::icon_mic => '\u{e9b2}',
            IconName::icon_hash => '\u{e9c7}',
        }
    }
}

/// Get icon for corresponding file type based on file path
pub fn icon_name_from_filepath(filepath: &str) -> IconName {
    // Extract file extension (without dot)
    let ext = filepath
        .rfind('.')
        .map(|i| &filepath[i + 1..])
        .unwrap_or("")
        .to_lowercase();

    // Return corresponding icon based on extension
    match ext.as_str() {
        // Document types
        "pdf" => IconName::icon_file_pdf1,
        "doc" | "docx" => IconName::icon_file_docx1,
        "xls" | "xlsx" => IconName::icon_file_xlsx1,
        "ppt" | "pptx" => IconName::icon_file_ppt1,
        "odt" => IconName::icon_file_odt1,
        "ods" => IconName::icon_file_ods1,
        "odp" => IconName::icon_file_odp1,
        "ott" => IconName::icon_file_ott1,
        "ots" => IconName::icon_file_ots1,
        "dot" | "dotx" => IconName::icon_file_dotx1,
        "pps" => IconName::icon_file_pps1,
        "rtf" => IconName::icon_file_rtf1,
        "txt" => IconName::icon_file_txt1,
        "md" | "markdown" => IconName::icon_document_text,
        
        // Image types
        "jpg" | "jpeg" => IconName::icon_file_jpg1,
        "png" => IconName::icon_file_png1,
        "gif" => IconName::icon_file_gif1,
        "bmp" => IconName::icon_file_bmp1,
        "tiff" | "tif" => IconName::icon_file_tiff1,
        "tga" => IconName::icon_file_tga1,
        "psd" => IconName::icon_file_psd1,
        "ai" => IconName::icon_file_ai1,
        "eps" => IconName::icon_file_eps1,
        "dwg" => IconName::icon_file_dwg1,
        "dxf" => IconName::icon_file_dxf1,
        
        // Audio types
        "mp3" => IconName::icon_file_mp31,
        "wav" => IconName::icon_file_wav1,
        "aac" => IconName::icon_file_aac1,
        "aiff" | "aif" => IconName::icon_file_aiff1,
        "mid" | "midi" => IconName::icon_file_mid1,
        
        // Video types
        "mp4" => IconName::icon_file_mp41,
        "avi" => IconName::icon_file_avi1,
        "mov" => IconName::icon_file_mov1,
        "m4v" => IconName::icon_file_m4v1,
        "mpg" | "mpeg" => IconName::icon_file_mpg1,
        "flv" => IconName::icon_file_flv1,
        "3gp" => IconName::icon_file_3gp1,
        "quicktime" | "qt" => IconName::icon_file_quicktime1,
        
        // Archive files
        "zip" => IconName::icon_file_zip1,
        "rar" => IconName::icon_file_rar1,
        "tgz" | "tar.gz" => IconName::icon_file_tgz1,
        "dmg" => IconName::icon_file_dmg1,
        "iso" => IconName::icon_file_iso1,
        
        // Code files
        "c" => IconName::icon_file_c1,
        "cpp" | "cc" | "cxx" => IconName::icon_file_cpp1,
        "h" | "hpp" => IconName::icon_file_h1,
        "py" => IconName::icon_file_py1,
        "java" => IconName::icon_file_java1,
        "rb" => IconName::icon_file_rb1,
        "php" => IconName::icon_file_php1,
        "sql" => IconName::icon_file_sql1,
        "html" | "htm" => IconName::icon_file_html1,
        "css" => IconName::icon_file_css1,
        "xml" => IconName::icon_file_xml1,
        "js" | "javascript" => IconName::icon_file_txt1,
        "ts" | "typescript" => IconName::icon_file_txt1,
        "json" => IconName::icon_file_txt1,
        "yaml" | "yml" => IconName::icon_file_yml,
        "rs" => IconName::icon_file_txt1, // Rust files
        "go" => IconName::icon_file_txt1,
        "sh" | "bash" => IconName::icon_file_txt1,
        "asp" => IconName::icon_file_asp1,
        
        // Other file types
        "exe" => IconName::icon_file_exe1,
        "app" => IconName::icon_file_app1,
        "dat" => IconName::icon_file_dat1,
        "ics" => IconName::icon_file_ics1,
        "key" => IconName::icon_file_key1,
        
        // Default: generic file icon
        _ => IconName::icon_file_txt1,
    }
}